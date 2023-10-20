use anyhow::{anyhow, Context};
use async_trait::async_trait;
use slog_scope::{debug, info, warn};
use std::{path::Path, path::PathBuf, sync::Arc};

use mithril_common::entities::{
    Beacon, Certificate, CertificatePending, Epoch, ProtocolMessage, ProtocolMessagePartKey,
    SignedEntityType,
};
use mithril_common::store::StakeStorer;
use mithril_common::{CardanoNetwork, StdResult};

use crate::entities::OpenMessage;
use crate::{DependencyContainer, ProtocolError};

#[cfg(test)]
use mockall::automock;

use super::error::RunnerError;

/// Configuration structure dedicated to the AggregatorRuntime.
#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    /// Interval between each snapshot, in ms
    pub interval: u64,

    /// Cardano network
    pub network: CardanoNetwork,

    /// DB directory to snapshot
    pub db_directory: PathBuf,
}

impl AggregatorConfig {
    /// Create a new instance of AggregatorConfig.
    pub fn new(interval: u64, network: CardanoNetwork, db_directory: &Path) -> Self {
        Self {
            interval,
            network,
            db_directory: db_directory.to_path_buf(),
        }
    }
}

/// This trait is intended to allow mocking the AggregatorRunner in tests.
/// It exposes all the methods needed by the state machine.
#[async_trait]
pub trait AggregatorRunnerTrait: Sync + Send {
    /// Return the current beacon from the chain
    async fn get_beacon_from_chain(&self) -> StdResult<Beacon>;

    /// Retrieves the current non certified open message for a given signed entity type.
    async fn get_current_non_certified_open_message_for_signed_entity_type(
        &self,
        signed_entity_type: &SignedEntityType,
    ) -> StdResult<Option<OpenMessage>>;

    /// Retrieves the current non certified open message.
    async fn get_current_non_certified_open_message(&self) -> StdResult<Option<OpenMessage>>;

    /// Check if a certificate chain is valid.
    async fn is_certificate_chain_valid(&self, beacon: &Beacon) -> StdResult<()>;

    /// Update the multisigner with the given beacon.
    async fn update_beacon(&self, new_beacon: &Beacon) -> StdResult<()>;

    /// Read the stake distribution from the blockchain and store it.
    async fn update_stake_distribution(&self, new_beacon: &Beacon) -> StdResult<()>;

    /// Open the signer registration round of an epoch.
    async fn open_signer_registration_round(&self, new_beacon: &Beacon) -> StdResult<()>;

    /// Close the signer registration round of an epoch.
    async fn close_signer_registration_round(&self) -> StdResult<()>;

    /// Update the multisigner with the protocol parameters from configuration.
    async fn update_protocol_parameters_in_multisigner(&self, new_beacon: &Beacon)
        -> StdResult<()>;

    /// Compute the protocol message
    async fn compute_protocol_message(
        &self,
        signed_entity_type: &SignedEntityType,
    ) -> StdResult<ProtocolMessage>;

    /// Return the actual pending certificate from the multisigner.
    async fn create_new_pending_certificate_from_multisigner(
        &self,
        beacon: Beacon,
        signed_entity_type: &SignedEntityType,
    ) -> StdResult<CertificatePending>;

    /// Store the given pending certificate.
    async fn save_pending_certificate(
        &self,
        pending_certificate: CertificatePending,
    ) -> StdResult<()>;

    /// Drop the multisigner's actual pending certificate.
    async fn drop_pending_certificate(&self) -> StdResult<Option<CertificatePending>>;

    /// Create multi-signature.
    async fn create_certificate(
        &self,
        signed_entity_type: &SignedEntityType,
    ) -> StdResult<Option<Certificate>>;

    /// Create an artifact and persist it.
    async fn create_artifact(
        &self,
        signed_entity_type: &SignedEntityType,
        certificate: &Certificate,
    ) -> StdResult<()>;

    /// Update the EraChecker with EraReader information.
    async fn update_era_checker(&self, beacon: &Beacon) -> StdResult<()>;

    /// Certifier inform new epoch
    async fn inform_new_epoch(&self, epoch: Epoch) -> StdResult<()>;

    /// Create new open message
    async fn create_open_message(
        &self,
        signed_entity_type: &SignedEntityType,
        protocol_message: &ProtocolMessage,
    ) -> StdResult<OpenMessage>;
}

/// The runner responsibility is to expose a code API for the state machine. It
/// holds services and configuration.
pub struct AggregatorRunner {
    dependencies: Arc<DependencyContainer>,
}

impl AggregatorRunner {
    /// Create a new instance of the Aggrergator Runner.
    pub fn new(dependencies: Arc<DependencyContainer>) -> Self {
        Self { dependencies }
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
impl AggregatorRunnerTrait for AggregatorRunner {
    /// Return the current beacon from the chain
    async fn get_beacon_from_chain(&self) -> StdResult<Beacon> {
        debug!("RUNNER: get beacon from chain");
        let beacon = self
            .dependencies
            .beacon_provider
            .get_current_beacon()
            .await?;

        Ok(beacon)
    }

    async fn get_current_non_certified_open_message_for_signed_entity_type(
        &self,
        signed_entity_type: &SignedEntityType,
    ) -> StdResult<Option<OpenMessage>> {
        debug!("RUNNER: get_current_non_certified_open_message_for_signed_entity_type"; "signed_entity_type" => ?signed_entity_type);
        let open_message_maybe = match self
            .dependencies
            .certifier_service
            .get_open_message(signed_entity_type)
            .await
            .with_context(|| format!("CertifierService can not get open message for signed_entity_type: '{signed_entity_type}'"))?
        {
            Some(existing_open_message) if !existing_open_message.is_certified => {
                info!(
                    "RUNNER: get_current_non_certified_open_message_for_signed_entity_type: existing open message found, not already certified";
                    "signed_entity_type" => ?signed_entity_type
                );

                Some(existing_open_message)
            }
            Some(_) => {
                info!(
                    "RUNNER: get_current_non_certified_open_message_for_signed_entity_type: existing open message found, already certified";
                    "signed_entity_type" => ?signed_entity_type
                );

                None
            }
            None => {
                info!(
                    "RUNNER: get_current_non_certified_open_message_for_signed_entity_type: no open message found, a new one will be created";
                    "signed_entity_type" => ?signed_entity_type
                );
                // In order to fix flakiness in the end to end tests, we are
                // compelled to update the beacon (of the multi-signer) This
                // avoids the computation of the next AVK on the wrong epoch (in
                // 'compute_protocol_message')
                // TODO: remove 'current_beacon' from the multi-signer state
                // which will avoid this fix
                let chain_beacon: Beacon = self.get_beacon_from_chain().await?;
                self.update_beacon(&chain_beacon).await?;
                let protocol_message = self.compute_protocol_message(signed_entity_type).await.with_context(|| format!("AggregatorRunner can not compute protocol message for signed_entity_type: '{signed_entity_type}'"))?;

                Some(
                    self.create_open_message(signed_entity_type, &protocol_message)
                        .await
                        .with_context(|| format!("AggregatorRunner can not create open message for signed_entity_type: '{signed_entity_type}'"))?,
                )
            }
        };

        Ok(open_message_maybe)
    }

    async fn get_current_non_certified_open_message(&self) -> StdResult<Option<OpenMessage>> {
        debug!("RUNNER: get_current_non_certified_open_message");
        let signed_entity_types = vec![
            SignedEntityType::MithrilStakeDistribution(
                self.dependencies
                    .ticker_service
                    .get_current_epoch()
                    .await
                    .with_context(|| "DependencyContainer can not get current epoch")?,
            ),
            SignedEntityType::CardanoImmutableFilesFull(
                self.dependencies
                    .ticker_service
                    .get_current_immutable_beacon()
                    .await
                    .with_context(|| "DependencyContainer can not get current immutable beacon")?,
            ),
        ];

        for signed_entity_type in signed_entity_types {
            if let Some(open_message) = self
                .get_current_non_certified_open_message_for_signed_entity_type(&signed_entity_type)
                .await
                .with_context(|| format!("AggregatorRunner can not get current non certified open message for signed entity type: '{}'", &signed_entity_type))?
            {
                return Ok(Some(open_message));
            }
            info!(
                "RUNNER: get_current_non_certified_open_message: open message already certified";
                "signed_entity_type" => ?signed_entity_type
            );
        }

        Ok(None)
    }

    async fn is_certificate_chain_valid(&self, beacon: &Beacon) -> StdResult<()> {
        debug!("RUNNER: is_certificate_chain_valid");
        self.dependencies
            .certifier_service
            .verify_certificate_chain(beacon.epoch)
            .await?;

        Ok(())
    }

    // TODO: is this still useful?
    async fn update_beacon(&self, new_beacon: &Beacon) -> StdResult<()> {
        debug!("RUNNER: update beacon"; "beacon" => #?new_beacon);
        self.dependencies
            .multi_signer
            .write()
            .await
            .update_current_epoch(new_beacon.epoch)
            .await?;
        Ok(())
    }

    async fn update_stake_distribution(&self, new_beacon: &Beacon) -> StdResult<()> {
        debug!("RUNNER: update stake distribution"; "beacon" => #?new_beacon);
        self.dependencies
            .stake_distribution_service
            .update_stake_distribution()
            .await
            .with_context(|| format!("AggregatorRunner could not update stake distribution for beacon: '{new_beacon}'"))
    }

    async fn open_signer_registration_round(&self, new_beacon: &Beacon) -> StdResult<()> {
        debug!("RUNNER: open signer registration round"; "beacon" => #?new_beacon);
        let registration_epoch = new_beacon.epoch.offset_to_recording_epoch();

        let stakes = self
            .dependencies
            .stake_store
            .get_stakes(registration_epoch)
            .await?
            .unwrap_or_default();

        self.dependencies
            .signer_registration_round_opener
            .open_registration_round(registration_epoch, stakes)
            .await
    }

    async fn close_signer_registration_round(&self) -> StdResult<()> {
        debug!("RUNNER: close signer registration round");

        self.dependencies
            .signer_registration_round_opener
            .close_registration_round()
            .await
    }

    async fn update_protocol_parameters_in_multisigner(
        &self,
        new_beacon: &Beacon,
    ) -> StdResult<()> {
        debug!("RUNNER: update protocol parameters"; "beacon" => #?new_beacon);
        let protocol_parameters = self.dependencies.config.protocol_parameters.clone();

        self.update_beacon(new_beacon)
            .await
            .with_context(|| format!("AggregatorRunner can not update beacon: '{new_beacon}'"))?;
        self.dependencies
            .multi_signer
            .write()
            .await
            .update_protocol_parameters(&protocol_parameters.clone().into())
            .await
            .map_err(|e| anyhow!(e))
            .with_context(|| {
                format!(
                    "Multi Signer can not update protocol parameters: '{:?}'",
                    &protocol_parameters
                )
            })
    }

    async fn compute_protocol_message(
        &self,
        signed_entity_type: &SignedEntityType,
    ) -> StdResult<ProtocolMessage> {
        debug!("RUNNER: compute protocol message");
        let mut protocol_message = self
            .dependencies
            .signable_builder_service
            .compute_protocol_message(signed_entity_type.to_owned())
            .await
            .with_context(|| format!("Runner can not compute protocol message for signed entity type: '{signed_entity_type}'"))?;

        let multi_signer = self.dependencies.multi_signer.write().await;
        protocol_message.set_message_part(
            ProtocolMessagePartKey::NextAggregateVerificationKey,
            multi_signer
                .compute_next_stake_distribution_aggregate_verification_key()
                .await?
                .to_json_hex()
                .with_context(|| "convert next avk to json hex failure")?,
        );

        Ok(protocol_message)
    }

    async fn create_new_pending_certificate_from_multisigner(
        &self,
        beacon: Beacon,
        signed_entity_type: &SignedEntityType,
    ) -> StdResult<CertificatePending> {
        debug!("RUNNER: create new pending certificate from multisigner");
        let multi_signer = self.dependencies.multi_signer.read().await;

        let signers = match multi_signer.get_signers().await {
            Ok(signers) => signers,
            Err(ProtocolError::Epoch(_)) => vec![],
            Err(e) => return Err(e.into()),
        };
        let next_signers = match multi_signer.get_next_signers_with_stake().await {
            Ok(signers) => signers,
            Err(ProtocolError::Epoch(_)) => vec![],
            Err(e) => return Err(e.into()),
        };

        let protocol_parameters =
            multi_signer
                .get_protocol_parameters()
                .await?
                .ok_or_else(|| {
                    RunnerError::MissingProtocolParameters(format!(
                        "no current protocol parameters found for beacon {beacon:?}"
                    ))
                })?;

        let next_protocol_parameters = multi_signer
            .get_next_protocol_parameters()
            .await?
            .ok_or_else(|| {
                RunnerError::MissingProtocolParameters(format!(
                    "no next protocol parameters found for beacon {beacon:?}"
                ))
            })?;

        let pending_certificate = CertificatePending::new(
            beacon,
            signed_entity_type.to_owned(),
            protocol_parameters.into(),
            next_protocol_parameters.into(),
            signers,
            next_signers.into_iter().map(|s| s.into()).collect(),
        );

        Ok(pending_certificate)
    }

    async fn save_pending_certificate(
        &self,
        pending_certificate: CertificatePending,
    ) -> StdResult<()> {
        debug!("RUNNER: saving pending certificate");

        let signed_entity_type = pending_certificate.signed_entity_type.clone();
        self.dependencies
            .certificate_pending_store
            .save(pending_certificate)
            .await
            .map_err(|e| anyhow!(e))
            .with_context(|| format!("CertificatePendingStore can not save pending certificate with signed_entity_type: '{signed_entity_type}'"))
    }

    async fn drop_pending_certificate(&self) -> StdResult<Option<CertificatePending>> {
        debug!("RUNNER: drop pending certificate");

        let certificate_pending = self.dependencies.certificate_pending_store.remove().await?;
        if certificate_pending.is_none() {
            warn!(" > drop_pending_certificate::no certificate pending in store, did the previous loop crashed ?");
        }

        Ok(certificate_pending)
    }

    async fn create_certificate(
        &self,
        signed_entity_type: &SignedEntityType,
    ) -> StdResult<Option<Certificate>> {
        debug!("RUNNER: create multi-signature");

        self.dependencies
            .certifier_service
            .create_certificate(signed_entity_type)
            .await
            .with_context(|| {
                format!(
                    "CertifierService can not create certificate for signed_entity_type: '{signed_entity_type}'"
                )
            })
    }

    async fn create_artifact(
        &self,
        signed_entity_type: &SignedEntityType,
        certificate: &Certificate,
    ) -> StdResult<()> {
        debug!("RUNNER: create artifact");
        self.dependencies
            .signed_entity_service
            .create_artifact(signed_entity_type.to_owned(), certificate)
            .await
            .with_context(|| {
                format!(
                    "SignedEntityService can not create artifact for signed_entity_type: '{signed_entity_type}' with certificate hash: '{}'",
                    certificate.hash
                )
            })?;

        Ok(())
    }

    async fn update_era_checker(&self, beacon: &Beacon) -> StdResult<()> {
        let token = self
            .dependencies
            .era_reader
            .read_era_epoch_token(beacon.epoch)
            .await
            .with_context(|| {
                format!(
                    "EraReader can not get era epoch token for current epoch: '{}'",
                    beacon.epoch
                )
            })?;

        let current_era = token
            .get_current_supported_era()
            .with_context(|| "EraEpochToken can not get current supported era")?;
        self.dependencies
            .era_checker
            .change_era(current_era, token.get_current_epoch());
        debug!(
            "Current Era is {} (Epoch {}).",
            current_era,
            token.get_current_epoch()
        );

        if token.get_next_supported_era().is_err() {
            let era_name = &token.get_next_era_marker().unwrap().name;
            warn!("Upcoming Era '{era_name}' is not supported by this version of the software. Please update!");
        }

        Ok(())
    }

    async fn inform_new_epoch(&self, epoch: Epoch) -> StdResult<()> {
        self.dependencies
            .certifier_service
            .inform_epoch(epoch)
            .await?;
        self.dependencies
            .epoch_service
            .write()
            .await
            .inform_epoch(epoch)
            .await?;

        Ok(())
    }

    async fn create_open_message(
        &self,
        signed_entity_type: &SignedEntityType,
        protocol_message: &ProtocolMessage,
    ) -> StdResult<OpenMessage> {
        self.dependencies
            .certifier_service
            .create_open_message(signed_entity_type, protocol_message)
            .await
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        entities::OpenMessage,
        initialize_dependencies,
        runtime::{AggregatorRunner, AggregatorRunnerTrait},
        services::{MithrilStakeDistributionService, MockCertifierService},
        DependencyContainer, MithrilSignerRegisterer, SignerRegistrationRound,
    };
    use async_trait::async_trait;
    use mithril_common::{
        chain_observer::FakeObserver,
        digesters::DumbImmutableFileObserver,
        entities::{
            Beacon, CertificatePending, ProtocolMessage, SignedEntityType, StakeDistribution,
        },
        signable_builder::SignableBuilderService,
        store::StakeStorer,
        test_utils::{fake_data, MithrilFixtureBuilder},
        BeaconProviderImpl, CardanoNetwork, StdResult,
    };
    use mockall::{mock, predicate::eq};
    use std::sync::Arc;

    mock! {
        SignableBuilderServiceImpl { }

        #[async_trait]
        impl SignableBuilderService for SignableBuilderServiceImpl
        {

            async fn compute_protocol_message(
                &self,
                signed_entity_type: SignedEntityType,
            ) -> StdResult<ProtocolMessage>;
        }
    }

    async fn build_runner_with_fixture_data(deps: DependencyContainer) -> AggregatorRunner {
        let fixture = MithrilFixtureBuilder::default().with_signers(5).build();
        let current_epoch = deps
            .chain_observer
            .get_current_epoch()
            .await
            .unwrap()
            .unwrap();
        deps.init_state_from_fixture(
            &fixture,
            &[
                current_epoch.offset_to_signer_retrieval_epoch().unwrap(),
                current_epoch,
            ],
        )
        .await;

        let runner = AggregatorRunner::new(Arc::new(deps));
        let beacon = runner.get_beacon_from_chain().await.unwrap();
        runner.update_beacon(&beacon).await.unwrap();

        runner
    }

    #[tokio::test]
    async fn test_get_beacon_from_chain() {
        let expected_beacon = Beacon::new("private".to_string(), 2, 17);
        let mut dependencies = initialize_dependencies().await;
        let immutable_file_observer = Arc::new(DumbImmutableFileObserver::default());
        immutable_file_observer
            .shall_return(Some(expected_beacon.immutable_file_number))
            .await;
        let beacon_provider = Arc::new(BeaconProviderImpl::new(
            Arc::new(FakeObserver::new(Some(expected_beacon.clone()))),
            immutable_file_observer,
            CardanoNetwork::TestNet(42),
        ));
        dependencies.beacon_provider = beacon_provider;
        let runner = AggregatorRunner::new(Arc::new(dependencies));

        // Retrieves the expected beacon
        let res = runner.get_beacon_from_chain().await;
        assert_eq!(expected_beacon, res.unwrap());
    }

    #[tokio::test]
    async fn test_update_beacon() {
        let deps = initialize_dependencies().await;
        let deps = Arc::new(deps);
        let runner = AggregatorRunner::new(deps.clone());
        let beacon = runner.get_beacon_from_chain().await.unwrap();
        let res = runner.update_beacon(&beacon).await;

        assert!(res.is_ok());
        let stored_epoch = deps
            .multi_signer
            .read()
            .await
            .get_current_epoch()
            .await
            .unwrap();

        assert_eq!(beacon.epoch, stored_epoch);
    }

    #[tokio::test]
    async fn test_update_stake_distribution() {
        let mut deps = initialize_dependencies().await;
        let chain_observer = Arc::new(FakeObserver::default());
        deps.chain_observer = chain_observer.clone();
        deps.stake_distribution_service = Arc::new(MithrilStakeDistributionService::new(
            deps.stake_store.clone(),
            chain_observer.clone(),
        ));
        let deps = Arc::new(deps);
        let runner = AggregatorRunner::new(deps.clone());
        let beacon = runner.get_beacon_from_chain().await.unwrap();
        let fixture = MithrilFixtureBuilder::default().with_signers(5).build();
        let expected = fixture.stake_distribution();

        chain_observer
            .set_signers(fixture.signers_with_stake())
            .await;
        runner
            .update_stake_distribution(&beacon)
            .await
            .expect("updating stake distribution should not return an error");

        let saved_stake_distribution = deps
            .stake_store
            .get_stakes(beacon.epoch.offset_to_recording_epoch())
            .await
            .unwrap()
            .unwrap_or_else(|| {
                panic!(
                    "I should have a stake distribution for the epoch {:?}",
                    beacon.epoch
                )
            });

        assert_eq!(expected, saved_stake_distribution);
    }

    #[tokio::test]
    async fn test_open_signer_registration_round() {
        let mut deps = initialize_dependencies().await;
        let signer_registration_round_opener = Arc::new(MithrilSignerRegisterer::new(
            deps.chain_observer.clone(),
            deps.verification_key_store.clone(),
            deps.signer_recorder.clone(),
            None,
        ));
        deps.signer_registration_round_opener = signer_registration_round_opener.clone();
        let stake_store = deps.stake_store.clone();
        let deps = Arc::new(deps);
        let runner = AggregatorRunner::new(deps.clone());

        let beacon = fake_data::beacon();
        let recording_epoch = beacon.epoch.offset_to_recording_epoch();
        let stake_distribution: StakeDistribution =
            StakeDistribution::from([("a".to_string(), 5), ("b".to_string(), 10)]);

        stake_store
            .save_stakes(recording_epoch, stake_distribution.clone())
            .await
            .expect("Save Stake distribution should not fail");

        runner
            .open_signer_registration_round(&beacon)
            .await
            .expect("opening signer registration should not return an error");

        let saved_current_round = signer_registration_round_opener.get_current_round().await;

        let expected_signer_registration_round =
            SignerRegistrationRound::dummy(recording_epoch, stake_distribution);

        assert_eq!(
            Some(expected_signer_registration_round),
            saved_current_round,
        );
    }

    #[tokio::test]
    async fn test_close_signer_registration_round() {
        let mut deps = initialize_dependencies().await;
        let signer_registration_round_opener = Arc::new(MithrilSignerRegisterer::new(
            deps.chain_observer.clone(),
            deps.verification_key_store.clone(),
            deps.signer_recorder.clone(),
            None,
        ));
        deps.signer_registration_round_opener = signer_registration_round_opener.clone();
        let deps = Arc::new(deps);
        let runner = AggregatorRunner::new(deps.clone());

        let beacon = fake_data::beacon();
        runner
            .open_signer_registration_round(&beacon)
            .await
            .expect("opening signer registration should not return an error");

        runner
            .close_signer_registration_round()
            .await
            .expect("closing signer registration should not return an error");

        let saved_current_round = signer_registration_round_opener.get_current_round().await;
        assert!(saved_current_round.is_none());
    }

    #[tokio::test]
    async fn test_update_protocol_parameters_in_multisigner() {
        let deps = initialize_dependencies().await;
        let deps = Arc::new(deps);
        let runner = AggregatorRunner::new(deps.clone());
        let beacon = runner.get_beacon_from_chain().await.unwrap();
        runner
            .update_beacon(&beacon)
            .await
            .expect("setting the beacon should not fail");
        runner
            .update_protocol_parameters_in_multisigner(&beacon)
            .await
            .expect("updating protocol parameters should not return an error");

        let current_protocol_parameters = deps.config.protocol_parameters.clone();

        let saved_protocol_parameters = deps
            .protocol_parameters_store
            .get_protocol_parameters(beacon.epoch.offset_to_protocol_parameters_recording_epoch())
            .await
            .unwrap()
            .unwrap_or_else(|| {
                panic!(
                    "should have protocol parameters for the epoch {:?}",
                    beacon.epoch
                )
            });

        assert_eq!(current_protocol_parameters, saved_protocol_parameters);
    }

    #[tokio::test]
    async fn test_create_new_pending_certificate_from_multisigner() {
        let deps = initialize_dependencies().await;
        let deps = Arc::new(deps);
        let runner = AggregatorRunner::new(deps.clone());
        let beacon = runner.get_beacon_from_chain().await.unwrap();
        runner.update_beacon(&beacon).await.unwrap();
        let signed_entity_type = SignedEntityType::dummy();

        let fixture = MithrilFixtureBuilder::default().with_signers(5).build();
        let current_signers = fixture.signers_with_stake()[1..3].to_vec();
        let next_signers = fixture.signers_with_stake()[2..5].to_vec();
        let protocol_parameters = fake_data::protocol_parameters();
        deps.prepare_for_genesis(
            current_signers.clone(),
            next_signers.clone(),
            &protocol_parameters.clone(),
        )
        .await;

        let mut certificate = runner
            .create_new_pending_certificate_from_multisigner(beacon.clone(), &signed_entity_type)
            .await
            .unwrap();
        certificate.signers.sort_by_key(|s| s.party_id.clone());
        certificate.next_signers.sort_by_key(|s| s.party_id.clone());
        let mut expected = CertificatePending::new(
            beacon,
            signed_entity_type,
            protocol_parameters.clone(),
            protocol_parameters,
            current_signers.into_iter().map(|s| s.into()).collect(),
            next_signers.into_iter().map(|s| s.into()).collect(),
        );
        expected.signers.sort_by_key(|s| s.party_id.clone());
        expected.next_signers.sort_by_key(|s| s.party_id.clone());

        assert_eq!(expected, certificate);
    }

    #[tokio::test]
    async fn test_save_pending_certificate() {
        let deps = initialize_dependencies().await;
        let deps = Arc::new(deps);
        let runner = AggregatorRunner::new(deps.clone());
        let beacon = runner.get_beacon_from_chain().await.unwrap();
        runner.update_beacon(&beacon).await.unwrap();
        let pending_certificate = fake_data::certificate_pending();

        runner
            .save_pending_certificate(pending_certificate.clone())
            .await
            .expect("save_pending_certificate should not fail");

        let saved_cert = deps.certificate_pending_store.get().await.unwrap().unwrap();
        assert_eq!(pending_certificate, saved_cert);
    }

    #[tokio::test]
    async fn test_drop_pending_certificate() {
        let deps = initialize_dependencies().await;
        let deps = Arc::new(deps);
        let runner = AggregatorRunner::new(deps.clone());
        let beacon = runner.get_beacon_from_chain().await.unwrap();
        runner.update_beacon(&beacon).await.unwrap();
        let pending_certificate = fake_data::certificate_pending();
        deps.certificate_pending_store
            .save(pending_certificate.clone())
            .await
            .unwrap();

        let cert = runner.drop_pending_certificate().await.unwrap();
        assert_eq!(Some(pending_certificate), cert);
        let maybe_saved_cert = deps.certificate_pending_store.get().await.unwrap();
        assert_eq!(None, maybe_saved_cert);
    }

    #[tokio::test]
    async fn test_drop_pending_no_certificate() {
        let deps = initialize_dependencies().await;
        let deps = Arc::new(deps);
        let runner = AggregatorRunner::new(deps.clone());
        let beacon = runner.get_beacon_from_chain().await.unwrap();
        runner.update_beacon(&beacon).await.unwrap();

        let cert = runner.drop_pending_certificate().await.unwrap();
        assert_eq!(None, cert);
        let maybe_saved_cert = deps.certificate_pending_store.get().await.unwrap();
        assert_eq!(None, maybe_saved_cert);
    }

    #[tokio::test]
    async fn test_update_era_checker() {
        let deps = initialize_dependencies().await;
        let beacon_provider = deps.beacon_provider.clone();
        let era_checker = deps.era_checker.clone();
        let mut beacon = beacon_provider.get_current_beacon().await.unwrap();

        assert_eq!(beacon.epoch, era_checker.current_epoch());
        let runner = AggregatorRunner::new(Arc::new(deps));
        beacon.epoch += 1;

        runner.update_era_checker(&beacon).await.unwrap();
        assert_eq!(beacon.epoch, era_checker.current_epoch());
    }

    #[tokio::test]
    async fn test_inform_new_epoch() {
        let mut mock_certifier_service = MockCertifierService::new();
        mock_certifier_service
            .expect_inform_epoch()
            .returning(|_| Ok(()))
            .times(1);
        let mut deps = initialize_dependencies().await;
        let current_epoch = deps
            .chain_observer
            .get_current_epoch()
            .await
            .unwrap()
            .unwrap();
        deps.certifier_service = Arc::new(mock_certifier_service);
        let runner = build_runner_with_fixture_data(deps).await;

        runner.inform_new_epoch(current_epoch).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_current_non_certified_open_message_should_create_new_open_message_for_mithril_stake_distribution_if_none_exists(
    ) {
        let open_message_expected = OpenMessage {
            signed_entity_type: SignedEntityType::MithrilStakeDistribution(
                fake_data::beacon().epoch,
            ),
            is_certified: false,
            ..OpenMessage::dummy()
        };
        let open_message_clone = open_message_expected.clone();

        let mut mock_certifier_service = MockCertifierService::new();
        mock_certifier_service
            .expect_get_open_message()
            .return_once(|_| Ok(None))
            .times(1);
        mock_certifier_service
            .expect_create_open_message()
            .return_once(|_, _| Ok(open_message_clone))
            .times(1);

        let mut deps = initialize_dependencies().await;
        deps.certifier_service = Arc::new(mock_certifier_service);

        let runner = build_runner_with_fixture_data(deps).await;

        let open_message_returned = runner
            .get_current_non_certified_open_message()
            .await
            .unwrap();
        assert_eq!(Some(open_message_expected), open_message_returned);
    }

    #[tokio::test]
    async fn test_get_current_non_certified_open_message_should_return_existing_open_message_for_mithril_stake_distribution_if_already_exists(
    ) {
        let open_message_expected = OpenMessage {
            signed_entity_type: SignedEntityType::MithrilStakeDistribution(
                fake_data::beacon().epoch,
            ),
            is_certified: false,
            ..OpenMessage::dummy()
        };
        let open_message_clone = open_message_expected.clone();

        let mut mock_certifier_service = MockCertifierService::new();
        mock_certifier_service
            .expect_get_open_message()
            .return_once(|_| Ok(Some(open_message_clone)))
            .times(1);
        mock_certifier_service.expect_create_open_message().never();

        let mut deps = initialize_dependencies().await;
        deps.certifier_service = Arc::new(mock_certifier_service);

        let runner = build_runner_with_fixture_data(deps).await;

        let open_message_returned = runner
            .get_current_non_certified_open_message()
            .await
            .unwrap();
        assert_eq!(Some(open_message_expected), open_message_returned);
    }

    #[tokio::test]
    async fn test_get_current_non_certified_open_message_should_return_existing_open_message_for_cardano_immutables_if_already_exists_and_open_message_mithril_stake_distribution_already_certified(
    ) {
        let open_message_already_certified = OpenMessage {
            signed_entity_type: SignedEntityType::MithrilStakeDistribution(
                fake_data::beacon().epoch,
            ),
            is_certified: true,
            ..OpenMessage::dummy()
        };
        let open_message_expected = OpenMessage {
            signed_entity_type: SignedEntityType::CardanoImmutableFilesFull(fake_data::beacon()),
            is_certified: false,
            ..OpenMessage::dummy()
        };
        let open_message_clone = open_message_expected.clone();

        let mut mock_certifier_service = MockCertifierService::new();
        mock_certifier_service
            .expect_get_open_message()
            .return_once(|_| Ok(Some(open_message_already_certified)))
            .times(1);
        mock_certifier_service
            .expect_get_open_message()
            .return_once(|_| Ok(Some(open_message_clone)))
            .times(1);
        mock_certifier_service.expect_create_open_message().never();

        let mut deps = initialize_dependencies().await;
        deps.certifier_service = Arc::new(mock_certifier_service);

        let runner = build_runner_with_fixture_data(deps).await;

        let open_message_returned = runner
            .get_current_non_certified_open_message()
            .await
            .unwrap();
        assert_eq!(Some(open_message_expected), open_message_returned);
    }

    #[tokio::test]
    async fn test_get_current_non_certified_open_message_should_create_open_message_for_cardano_immutables_if_none_exists_and_open_message_mithril_stake_distribution_already_certified(
    ) {
        let open_message_already_certified = OpenMessage {
            signed_entity_type: SignedEntityType::MithrilStakeDistribution(
                fake_data::beacon().epoch,
            ),
            is_certified: true,
            ..OpenMessage::dummy()
        };
        let open_message_expected = OpenMessage {
            signed_entity_type: SignedEntityType::CardanoImmutableFilesFull(fake_data::beacon()),
            is_certified: false,
            ..OpenMessage::dummy()
        };
        let open_message_clone = open_message_expected.clone();

        let mut mock_certifier_service = MockCertifierService::new();
        mock_certifier_service
            .expect_get_open_message()
            .with(eq(open_message_already_certified
                .signed_entity_type
                .clone()))
            .return_once(|_| Ok(Some(open_message_already_certified)))
            .times(1);
        mock_certifier_service
            .expect_get_open_message()
            .return_once(|_| Ok(None))
            .times(1);
        mock_certifier_service
            .expect_create_open_message()
            .return_once(|_, _| Ok(open_message_clone))
            .times(1);

        let mut mock_signable_builder_service = MockSignableBuilderServiceImpl::new();
        mock_signable_builder_service
            .expect_compute_protocol_message()
            .return_once(|_| Ok(ProtocolMessage::default()));

        let mut deps = initialize_dependencies().await;
        deps.certifier_service = Arc::new(mock_certifier_service);
        deps.signable_builder_service = Arc::new(mock_signable_builder_service);

        let runner = build_runner_with_fixture_data(deps).await;

        let open_message_returned = runner
            .get_current_non_certified_open_message()
            .await
            .unwrap();
        assert_eq!(Some(open_message_expected), open_message_returned);
    }

    #[tokio::test]
    async fn test_get_current_non_certified_open_message_should_return_none_if_all_open_message_already_certified(
    ) {
        let open_message_already_certified_mithril_stake_distribution = OpenMessage {
            signed_entity_type: SignedEntityType::MithrilStakeDistribution(
                fake_data::beacon().epoch,
            ),
            is_certified: true,
            ..OpenMessage::dummy()
        };
        let open_message_already_certified_cardano_immutable_files = OpenMessage {
            signed_entity_type: SignedEntityType::CardanoImmutableFilesFull(fake_data::beacon()),
            is_certified: true,
            ..OpenMessage::dummy()
        };

        let mut mock_certifier_service = MockCertifierService::new();
        mock_certifier_service
            .expect_get_open_message()
            .return_once(|_| {
                Ok(Some(
                    open_message_already_certified_mithril_stake_distribution,
                ))
            })
            .times(1);
        mock_certifier_service
            .expect_get_open_message()
            .return_once(|_| Ok(Some(open_message_already_certified_cardano_immutable_files)))
            .times(1);
        mock_certifier_service.expect_create_open_message().never();

        let mut deps = initialize_dependencies().await;
        deps.certifier_service = Arc::new(mock_certifier_service);

        let runner = build_runner_with_fixture_data(deps).await;

        let open_message_returned = runner
            .get_current_non_certified_open_message()
            .await
            .unwrap();
        assert!(open_message_returned.is_none());
    }
}
