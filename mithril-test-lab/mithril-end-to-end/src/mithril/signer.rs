use crate::devnet::PoolNode;
use crate::utils::MithrilCommand;
use crate::DEVNET_MAGIC_ID;
use mithril_common::entities::PartyId;
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Child;

#[derive(Debug)]
pub struct Signer {
    party_id: PartyId,
    command: MithrilCommand,
    process: Option<Child>,
}

impl Signer {
    pub fn new(
        aggregator_endpoint: String,
        pool_node: &PoolNode,
        cardano_cli_path: &Path,
        work_dir: &Path,
        bin_dir: &Path,
    ) -> Result<Self, String> {
        let party_id = pool_node.party_id()?;
        let magic_id = DEVNET_MAGIC_ID.to_string();
        let data_stores_path = format!("./stores/signer-{}", party_id);
        let mut env = HashMap::from([
            ("NETWORK", "devnet"),
            ("RUN_INTERVAL", "400"),
            ("AGGREGATOR_ENDPOINT", &aggregator_endpoint),
            ("DB_DIRECTORY", pool_node.db_path.to_str().unwrap()),
            ("DATA_STORES_DIRECTORY", &data_stores_path),
            ("NETWORK_MAGIC", &magic_id),
            (
                "CARDANO_NODE_SOCKET_PATH",
                pool_node.socket_path.to_str().unwrap(),
            ),
            ("CARDANO_CLI_PATH", cardano_cli_path.to_str().unwrap()),
        ]);
        if pool_node.node_index % 2 == 0 {
            // 50% of signers with key certification
            env.insert("PARTY_ID", &party_id); // TODO: Should be removed once the signer certification is fully deployed
            env.insert(
                "KES_SECRET_KEY_PATH",
                pool_node.kes_secret_key_path.to_str().unwrap(),
            );
            env.insert(
                "OPERATIONAL_CERTIFICATE_PATH",
                pool_node.operational_certificate_path.to_str().unwrap(),
            );
        } else {
            // 50% of signers without key certification (legacy)
            // TODO: Should be removed once the signer certification is fully deployed
            env.insert("PARTY_ID", &party_id);
        }
        let args = vec!["-vvv"];

        let mut command = MithrilCommand::new("mithril-signer", work_dir, bin_dir, env, &args)?;
        command.set_log_name(format!("mithril-signer-{}", party_id).as_str());

        Ok(Self {
            party_id,
            command,
            process: None,
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.process = Some(self.command.start(&[])?);
        Ok(())
    }

    pub async fn tail_logs(&self, number_of_line: u64) -> Result<(), String> {
        self.command
            .tail_logs(
                Some(format!("mithril-signer-{}", self.party_id).as_str()),
                number_of_line,
            )
            .await
    }
}
