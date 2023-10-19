use anyhow::{anyhow, Context};
use async_trait::async_trait;
use slog_scope::{debug, warn};
use std::sync::Arc;
use thiserror::Error;

use mithril_common::{
    crypto_helper::{
        ProtocolAggregateVerificationKey, ProtocolAggregationError, ProtocolMultiSignature,
        ProtocolParameters, ProtocolStakeDistribution,
    },
    entities::{self, Epoch, SignerWithStake},
    protocol::{MultiSigner as ProtocolMultiSigner, SignerBuilder},
    store::StakeStorer,
    StdError,
};

use crate::{entities::OpenMessage, store::VerificationKeyStorer, ProtocolParametersStorer};

#[cfg(test)]
use mockall::automock;

/// Error type for multi signer service.
#[derive(Error, Debug)]
pub enum ProtocolError {
    /// Signer is already registered.
    #[error("signer already registered")]
    ExistingSigner,

    /// Signer was not registered.
    #[error("signer did not register")]
    UnregisteredParty,

    /// Signer registration failed.
    #[error("signer registration failed")]
    FailedSignerRegistration(#[source] StdError),

    /// Single signature already recorded.
    #[error("single signature already recorded")]
    ExistingSingleSignature(entities::PartyId),

    /// Mithril STM library returned an error.
    #[error("core error")]
    Core(#[source] StdError),

    /// No message available.
    #[error("no message available")]
    UnavailableMessage,

    /// No protocol parameters available.
    #[error("no protocol parameters available")]
    UnavailableProtocolParameters,

    /// No clerk available.
    #[error("no clerk available")]
    UnavailableClerk,

    /// No epoch available.
    #[error("no epoch available")]
    UnavailableEpoch,

    /// Store error.
    #[error("store error")]
    StoreError(#[source] StdError),

    /// Epoch error.
    #[error("epoch error")]
    Epoch(#[from] entities::EpochError),
}

/// MultiSigner is the cryptographic engine in charge of producing multi signatures from individual signatures
#[cfg_attr(test, automock)]
#[async_trait]
pub trait MultiSigner: Sync + Send {
    /// Get current epoch
    async fn get_current_epoch(&self) -> Option<Epoch>;

    /// Update current epoch
    async fn update_current_epoch(&mut self, epoch: Epoch) -> Result<(), ProtocolError>;

    /// Get protocol parameters
    async fn get_protocol_parameters(&self) -> Result<Option<ProtocolParameters>, ProtocolError>;

    /// Update protocol parameters
    async fn update_protocol_parameters(
        &mut self,
        protocol_parameters: &ProtocolParameters,
    ) -> Result<(), ProtocolError>;

    /// Get next protocol parameters
    async fn get_next_protocol_parameters(
        &self,
    ) -> Result<Option<ProtocolParameters>, ProtocolError>;

    /// Compute aggregate verification key from stake distribution
    async fn compute_aggregate_verification_key(
        &self,
        signers_with_stakes: &[SignerWithStake],
        protocol_parameters: &ProtocolParameters,
    ) -> Result<ProtocolAggregateVerificationKey, ProtocolError>;

    /// Compute stake distribution aggregate verification key
    async fn compute_stake_distribution_aggregate_verification_key(
        &self,
    ) -> Result<ProtocolAggregateVerificationKey, ProtocolError> {
        let signers_with_stake = self.get_signers_with_stake().await?;
        let protocol_parameters = self
            .get_protocol_parameters()
            .await?
            .ok_or(ProtocolError::UnavailableProtocolParameters)?;
        Ok(self
            .compute_aggregate_verification_key(&signers_with_stake, &protocol_parameters)
            .await?)
    }

    /// Compute next stake distribution aggregate verification key
    async fn compute_next_stake_distribution_aggregate_verification_key(
        &self,
    ) -> Result<ProtocolAggregateVerificationKey, ProtocolError> {
        let next_signers_with_stake = self.get_next_signers_with_stake().await?;
        let protocol_parameters = self
            .get_next_protocol_parameters()
            .await?
            .ok_or(ProtocolError::UnavailableProtocolParameters)?;
        Ok(self
            .compute_aggregate_verification_key(&next_signers_with_stake, &protocol_parameters)
            .await?)
    }

    /// Get signers
    async fn get_signers(&self) -> Result<Vec<entities::Signer>, ProtocolError> {
        debug!("Get signers");
        Ok(self
            .get_signers_with_stake()
            .await?
            .into_iter()
            .map(|signer| signer.into())
            .collect::<Vec<entities::Signer>>())
    }

    /// Get signers with stake
    async fn get_signers_with_stake(&self) -> Result<Vec<SignerWithStake>, ProtocolError>;

    /// Get signers for the next epoch with their stake
    async fn get_next_signers_with_stake(&self) -> Result<Vec<SignerWithStake>, ProtocolError>;

    /// Verify a single signature
    async fn verify_single_signature(
        &self,
        message: &entities::ProtocolMessage,
        signatures: &entities::SingleSignatures,
    ) -> Result<(), ProtocolError>;

    /// Creates a multi signature from single signatures
    async fn create_multi_signature(
        &self,
        open_message: &OpenMessage,
    ) -> Result<Option<ProtocolMultiSignature>, ProtocolError>;
}

/// MultiSignerImpl is an implementation of the MultiSigner
pub struct MultiSignerImpl {
    /// Epoch that is currently used
    current_epoch: Option<Epoch>,

    /// Verification key store
    verification_key_store: Arc<dyn VerificationKeyStorer>,

    /// Stake store
    stake_store: Arc<dyn StakeStorer>,

    /// Protocol parameters store
    protocol_parameters_store: Arc<dyn ProtocolParametersStorer>,
}

impl MultiSignerImpl {
    /// MultiSignerImpl factory
    pub fn new(
        verification_key_store: Arc<dyn VerificationKeyStorer>,
        stake_store: Arc<dyn StakeStorer>,
        protocol_parameters_store: Arc<dyn ProtocolParametersStorer>,
    ) -> Self {
        debug!("New MultiSignerImpl created");
        Self {
            current_epoch: None,
            verification_key_store,
            stake_store,
            protocol_parameters_store,
        }
    }

    /// Creates a protocol multi signer
    pub fn create_protocol_multi_signer(
        &self,
        signers_with_stake: &[SignerWithStake],
        protocol_parameters: &ProtocolParameters,
    ) -> Result<ProtocolMultiSigner, ProtocolError> {
        debug!("Create protocol_multi_signer");

        let protocol_multi_signer =
            SignerBuilder::new(signers_with_stake, &(*protocol_parameters).into())
                .with_context(|| "Multi Signer can not build a protocol multi signer")
                .map_err(|e| ProtocolError::Core(anyhow!(e)))?
                .build_multi_signer();

        Ok(protocol_multi_signer)
    }

    /// Get the [stake distribution][ProtocolStakeDistribution] for the given `epoch`
    async fn get_stake_distribution_at_epoch(
        &self,
        epoch: entities::Epoch,
    ) -> Result<ProtocolStakeDistribution, ProtocolError> {
        debug!("Get stake distribution at epoch"; "epoch"=> #?epoch);

        let stakes = self
            .stake_store
            .get_stakes(epoch)
            .await
            .with_context(|| format!("Multi Signer can not retrieve stakes for epoch '{epoch}'"))
            .map_err(|e| ProtocolError::StoreError(anyhow!(e)))?
            .unwrap_or_default();

        Ok(stakes.into_iter().collect::<ProtocolStakeDistribution>())
    }

    /// Get the [protocol parameters][ProtocolParameters] for the given `epoch`
    async fn get_protocol_parameters_at_epoch(
        &self,
        epoch: entities::Epoch,
    ) -> Result<Option<ProtocolParameters>, ProtocolError> {
        debug!("Get protocol parameters at epoch"; "epoch"=> #?epoch);

        match self
            .protocol_parameters_store
            .get_protocol_parameters(epoch)
            .await
        {
            Ok(Some(protocol_parameters)) => Ok(Some(protocol_parameters.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(ProtocolError::StoreError(anyhow!(e).context(
                "Multi Signer can not retrieve protocol parameters for epoch '{epoch}'",
            ))),
        }
    }

    /// Get stake distribution
    async fn get_stake_distribution(&self) -> Result<ProtocolStakeDistribution, ProtocolError> {
        debug!("Get stake distribution");
        let epoch = self
            .current_epoch
            .ok_or(ProtocolError::UnavailableEpoch)?
            .offset_to_signer_retrieval_epoch()?;
        self.get_stake_distribution_at_epoch(epoch).await
    }

    /// Get next stake distribution
    async fn get_next_stake_distribution(
        &self,
    ) -> Result<ProtocolStakeDistribution, ProtocolError> {
        debug!("Get next stake distribution");
        let epoch = self
            .current_epoch
            .ok_or(ProtocolError::UnavailableEpoch)?
            .offset_to_next_signer_retrieval_epoch();
        self.get_stake_distribution_at_epoch(epoch).await
    }
}

#[async_trait]
impl MultiSigner for MultiSignerImpl {
    async fn get_current_epoch(&self) -> Option<Epoch> {
        self.current_epoch
    }

    async fn update_current_epoch(&mut self, epoch: Epoch) -> Result<(), ProtocolError> {
        debug!("Update update_current_epoch to {:?}", epoch);
        self.current_epoch = Some(epoch);

        Ok(())
    }

    // TODO: protocol parameters should ALWAYS be available
    /// Get protocol parameters
    async fn get_protocol_parameters(&self) -> Result<Option<ProtocolParameters>, ProtocolError> {
        debug!("Get protocol parameters");
        let epoch = self
            .current_epoch
            .ok_or(ProtocolError::UnavailableEpoch)?
            .offset_to_signer_retrieval_epoch()?;
        self.get_protocol_parameters_at_epoch(epoch).await
    }

    /// Update protocol parameters
    async fn update_protocol_parameters(
        &mut self,
        protocol_parameters: &ProtocolParameters,
    ) -> Result<(), ProtocolError> {
        debug!("Update protocol parameters to {:?}", protocol_parameters);
        let epoch = self
            .current_epoch
            .ok_or(ProtocolError::UnavailableEpoch)?
            .offset_to_protocol_parameters_recording_epoch();

        self.protocol_parameters_store
            .save_protocol_parameters(epoch, protocol_parameters.to_owned().into())
            .await.with_context(|| format!("Multi Signer can not update protocol parameters '{protocol_parameters:?}' for epoch '{epoch}'"))
            .map_err(|e| ProtocolError::StoreError(anyhow!(e)))?;

        Ok(())
    }

    /// Get next protocol parameters
    async fn get_next_protocol_parameters(
        &self,
    ) -> Result<Option<ProtocolParameters>, ProtocolError> {
        debug!("Get next protocol parameters");
        let epoch = self
            .current_epoch
            .ok_or(ProtocolError::UnavailableEpoch)?
            .offset_to_next_signer_retrieval_epoch();
        self.get_protocol_parameters_at_epoch(epoch).await
    }

    /// Compute aggregate verification key from stake distribution
    async fn compute_aggregate_verification_key(
        &self,
        signers_with_stakes: &[SignerWithStake],
        protocol_parameters: &ProtocolParameters,
    ) -> Result<ProtocolAggregateVerificationKey, ProtocolError> {
        let protocol_multi_signer =
            self.create_protocol_multi_signer(signers_with_stakes, protocol_parameters)?;

        Ok(protocol_multi_signer.compute_aggregate_verification_key())
    }

    async fn get_signers_with_stake(&self) -> Result<Vec<SignerWithStake>, ProtocolError> {
        debug!("Get signers with stake");
        let epoch = self
            .current_epoch
            .ok_or(ProtocolError::UnavailableEpoch)?
            .offset_to_signer_retrieval_epoch()?;
        let signers = self
            .verification_key_store
            .get_verification_keys(epoch)
            .await
            .with_context(|| {
                format!(
                    "Multi Signer can not retrieve signers verification keys for epoch '{epoch}'"
                )
            })
            .map_err(|e| ProtocolError::StoreError(anyhow!(e)))?
            .unwrap_or_default();

        Ok(self
            .get_stake_distribution()
            .await?
            .iter()
            .filter_map(|(party_id, stake)| {
                signers.get(party_id).map(|signer| {
                    SignerWithStake::new(
                        party_id.to_owned(),
                        signer.verification_key.to_owned(),
                        signer.verification_key_signature.to_owned(),
                        signer.operational_certificate.to_owned(),
                        signer.kes_period.to_owned(),
                        *stake,
                    )
                })
            })
            .collect())
    }

    async fn get_next_signers_with_stake(&self) -> Result<Vec<SignerWithStake>, ProtocolError> {
        debug!("Get next signers with stake");
        let epoch = self
            .current_epoch
            .ok_or(ProtocolError::UnavailableEpoch)?
            .offset_to_next_signer_retrieval_epoch();
        let signers = self
            .verification_key_store
            .get_verification_keys(epoch)
            .await.with_context(|| {
                format!(
                    "Multi Signer can not retrieve next signers verification keys for epoch '{epoch}'"
                )
            })
            .map_err(|e| ProtocolError::StoreError(anyhow!(e)))?
            .unwrap_or_default();

        Ok(self
            .get_next_stake_distribution()
            .await?
            .iter()
            .filter_map(|(party_id, stake)| {
                signers.get(party_id).map(|signer| {
                    SignerWithStake::new(
                        party_id.to_owned(),
                        signer.verification_key.to_owned(),
                        signer.verification_key_signature.to_owned(),
                        signer.operational_certificate.to_owned(),
                        signer.kes_period.to_owned(),
                        *stake,
                    )
                })
            })
            .collect())
    }

    /// Verify a single signature
    async fn verify_single_signature(
        &self,
        message: &entities::ProtocolMessage,
        single_signature: &entities::SingleSignatures,
    ) -> Result<(), ProtocolError> {
        debug!(
            "Verify single signature from {} at indexes {:?} for message {:?}",
            single_signature.party_id, single_signature.won_indexes, message
        );

        let protocol_parameters = self
            .get_protocol_parameters()
            .await?
            .ok_or(ProtocolError::UnavailableProtocolParameters)?;

        let signers_with_stakes = self.get_signers_with_stake().await?;

        let protocol_multi_signer =
            self.create_protocol_multi_signer(&signers_with_stakes, &protocol_parameters)?;

        protocol_multi_signer
            .verify_single_signature(message, single_signature)
            .with_context(|| "Multi Signer can not verify single signature for message '{message}'")
            .map_err(|e| ProtocolError::Core(anyhow!(e)))
    }

    /// Creates a multi signature from single signatures
    async fn create_multi_signature(
        &self,
        open_message: &OpenMessage,
    ) -> Result<Option<ProtocolMultiSignature>, ProtocolError> {
        debug!("MultiSigner:create_multi_signature({open_message:?})");
        let protocol_parameters = self
            .get_protocol_parameters()
            .await?
            .ok_or(ProtocolError::UnavailableProtocolParameters)?;

        let signers_with_stakes = self.get_signers_with_stake().await?;

        let protocol_multi_signer =
            self.create_protocol_multi_signer(&signers_with_stakes, &protocol_parameters)?;

        match protocol_multi_signer.aggregate_single_signatures(
            &open_message.single_signatures,
            &open_message.protocol_message,
        ) {
            Ok(multi_signature) => Ok(Some(multi_signature)),
            Err(ProtocolAggregationError::NotEnoughSignatures(actual, expected)) => {
                warn!("Could not compute multi-signature: Not enough signatures. Got only {} out of {}.", actual, expected);
                Ok(None)
            }
            Err(err) => Err(ProtocolError::Core(anyhow!(err).context(format!(
                "Multi Signer can not create multi-signature for entity type '{:?}'",
                open_message.signed_entity_type
            )))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{store::VerificationKeyStore, ProtocolParametersStore};
    use mithril_common::{
        crypto_helper::tests_setup::*,
        entities::{Beacon, PartyId, SignedEntityType, StakeDistribution},
        store::{adapter::MemoryAdapter, StakeStore},
        test_utils::{fake_data, MithrilFixtureBuilder},
    };
    use std::{collections::HashMap, sync::Arc};

    async fn setup_multi_signer() -> MultiSignerImpl {
        let epoch = fake_data::beacon().epoch;
        let verification_key_store = VerificationKeyStore::new(Box::new(
            MemoryAdapter::<Epoch, HashMap<PartyId, SignerWithStake>>::new(None).unwrap(),
        ));
        let stake_store = StakeStore::new(
            Box::new(MemoryAdapter::<Epoch, StakeDistribution>::new(None).unwrap()),
            None,
        );
        let protocol_parameters_store = ProtocolParametersStore::new(
            Box::new(
                MemoryAdapter::<Epoch, entities::ProtocolParameters>::new(Some(vec![
                    (
                        epoch.offset_to_signer_retrieval_epoch().unwrap(),
                        fake_data::protocol_parameters(),
                    ),
                    (
                        epoch.offset_to_next_signer_retrieval_epoch(),
                        fake_data::protocol_parameters(),
                    ),
                    (
                        epoch.offset_to_next_signer_retrieval_epoch().next(),
                        fake_data::protocol_parameters(),
                    ),
                ]))
                .unwrap(),
            ),
            None,
        );
        let mut multi_signer = MultiSignerImpl::new(
            Arc::new(verification_key_store),
            Arc::new(stake_store),
            Arc::new(protocol_parameters_store),
        );

        multi_signer
            .update_current_epoch(epoch)
            .await
            .expect("update_current_beacon should not fail");
        multi_signer
    }

    async fn offset_epoch(multi_signer: &mut MultiSignerImpl, offset: i64) {
        let epoch = multi_signer.get_current_epoch().await.unwrap();
        let epoch_new = epoch.offset_by(offset).unwrap();
        multi_signer.update_current_epoch(epoch_new).await.unwrap();
    }

    fn take_signatures_until_quorum_is_almost_reached(
        signatures: &mut Vec<entities::SingleSignatures>,
        quorum: usize,
    ) -> Vec<entities::SingleSignatures> {
        signatures.sort_by(|l, r| l.won_indexes.len().cmp(&r.won_indexes.len()));

        let mut result = vec![];
        let mut nb_won_indexes = 0;

        while let Some(signature) = signatures.first() {
            if signature.won_indexes.len() + nb_won_indexes >= quorum {
                break;
            }
            nb_won_indexes += signature.won_indexes.len();
            result.push(signatures.remove(0));
        }

        result
    }

    #[tokio::test]
    async fn test_multi_signer_protocol_parameters_ok() {
        let mut multi_signer = setup_multi_signer().await;

        let protocol_parameters_expected = setup_protocol_parameters();
        multi_signer
            .update_protocol_parameters(&protocol_parameters_expected)
            .await
            .expect("update protocol parameters failed");

        offset_epoch(
            &mut multi_signer,
            Epoch::SIGNER_RECORDING_OFFSET as i64 - Epoch::SIGNER_RETRIEVAL_OFFSET,
        )
        .await;

        let protocol_parameters = multi_signer
            .get_protocol_parameters()
            .await
            .expect("protocol parameters should have been retrieved");
        let protocol_parameters: entities::ProtocolParameters = protocol_parameters.unwrap().into();
        let protocol_parameters_expected: entities::ProtocolParameters =
            protocol_parameters_expected.into();
        assert_eq!(protocol_parameters_expected, protocol_parameters);
    }

    #[tokio::test]
    async fn test_multi_signer_stake_distribution_ok() {
        let mut multi_signer = setup_multi_signer().await;
        let stake_store = multi_signer.stake_store.clone();
        let fixture = MithrilFixtureBuilder::default().with_signers(5).build();
        let mut stake_distribution_expected = fixture.protocol_stake_distribution();

        stake_distribution_expected.sort_by_key(|k| k.0.clone());
        stake_store
            .save_stakes(
                multi_signer
                    .current_epoch
                    .unwrap()
                    .offset_to_recording_epoch(),
                fixture.stake_distribution(),
            )
            .await
            .expect("update stake distribution failed");

        offset_epoch(
            &mut multi_signer,
            Epoch::SIGNER_RECORDING_OFFSET as i64 - Epoch::SIGNER_RETRIEVAL_OFFSET,
        )
        .await;

        let mut stake_distribution = multi_signer
            .get_stake_distribution()
            .await
            .expect("get state distribution failed");
        stake_distribution.sort_by_key(|k| k.0.clone());
        let stake_distribution_next_expected = multi_signer
            .get_next_stake_distribution()
            .await
            .expect("get next state distribution failed");
        assert_eq!(stake_distribution_expected, stake_distribution);

        offset_epoch(&mut multi_signer, 1).await;

        let mut stake_distribution = multi_signer
            .get_stake_distribution()
            .await
            .expect("get state distribution failed");
        stake_distribution.sort_by_key(|k| k.0.clone());
        assert_eq!(stake_distribution_next_expected, stake_distribution);
    }

    #[tokio::test]
    async fn test_multi_signer_multi_signature_ok() {
        let mut multi_signer = setup_multi_signer().await;
        let stake_store = multi_signer.stake_store.clone();
        let verification_key_store = multi_signer.verification_key_store.clone();
        let start_epoch = multi_signer.current_epoch.unwrap();

        let message = setup_message();
        let protocol_parameters = setup_protocol_parameters();
        multi_signer
            .update_protocol_parameters(&protocol_parameters)
            .await
            .expect("update protocol parameters failed");

        let fixture = MithrilFixtureBuilder::default().with_signers(5).build();

        stake_store
            .save_stakes(
                start_epoch.offset_to_recording_epoch(),
                fixture.stake_distribution(),
            )
            .await
            .expect("update stake distribution failed");
        for signer_with_stake in &fixture.signers_with_stake() {
            verification_key_store
                .save_verification_key(
                    start_epoch.offset_to_recording_epoch(),
                    signer_with_stake.to_owned(),
                )
                .await
                .expect("register should have succeeded");
        }

        offset_epoch(
            &mut multi_signer,
            Epoch::SIGNER_RECORDING_OFFSET as i64 - Epoch::SIGNER_RETRIEVAL_OFFSET,
        )
        .await;

        let mut signatures = Vec::new();

        let mut expected_certificate_signers: Vec<SignerWithStake> = Vec::new();
        for signer_fixture in fixture.signers_fixture() {
            if let Some(signature) = signer_fixture
                .protocol_signer
                .sign(message.compute_hash().as_bytes())
            {
                let won_indexes = signature.indexes.clone();

                signatures.push(entities::SingleSignatures::new(
                    signer_fixture.signer_with_stake.party_id.to_owned(),
                    signature.into(),
                    won_indexes,
                ));

                expected_certificate_signers.push(signer_fixture.signer_with_stake.to_owned())
            }
        }

        for signature in &signatures {
            multi_signer
                .verify_single_signature(&message, signature)
                .await
                .expect("single signature should be valid");
        }

        let signatures_to_almost_reach_quorum = take_signatures_until_quorum_is_almost_reached(
            &mut signatures,
            protocol_parameters.k as usize,
        );
        assert!(
            !signatures_to_almost_reach_quorum.is_empty(),
            "they should be at least one signature that can be registered without reaching the quorum"
        );

        let mut open_message = OpenMessage {
            epoch: start_epoch,
            signed_entity_type: SignedEntityType::CardanoImmutableFilesFull(Beacon {
                epoch: multi_signer.current_epoch.unwrap(),
                ..fake_data::beacon()
            }),
            protocol_message: message.clone(),
            is_certified: false,
            single_signatures: Vec::new(),
            ..OpenMessage::dummy()
        };

        // No signatures registered: multi-signer can't create the multi-signature
        assert!(multi_signer
            .create_multi_signature(&open_message)
            .await
            .expect("create multi signature should not fail")
            .is_none());

        // Add some signatures but not enough to reach the quorum: multi-signer should not create the multi-signature
        open_message.single_signatures = signatures_to_almost_reach_quorum;

        assert!(multi_signer
            .create_multi_signature(&open_message)
            .await
            .expect("create multi signature should not fail")
            .is_none());

        // Add the remaining signatures to reach the quorum: multi-signer should create a multi-signature
        open_message.single_signatures.append(&mut signatures);

        assert!(
            multi_signer
                .create_multi_signature(&open_message)
                .await
                .expect("create multi signature should not fail")
                .is_some(),
            "no multi-signature were computed"
        );
    }
}
