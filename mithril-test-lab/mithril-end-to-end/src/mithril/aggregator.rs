use crate::devnet::BftNode;
use crate::utils::MithrilCommand;
use crate::DEVNET_MAGIC_ID;
use anyhow::{anyhow, Context};
use mithril_common::{entities, StdResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Child;

#[derive(Debug)]
pub struct AggregatorConfig<'a> {
    pub server_port: u64,
    pub bft_node: &'a BftNode,
    pub cardano_cli_path: &'a Path,
    pub work_dir: &'a Path,
    pub bin_dir: &'a Path,
    pub mithril_era: &'a str,
    pub signed_entity_types: &'a [String],
    pub chain_observer_type: &'a str,
}

#[derive(Debug)]
pub struct Aggregator {
    server_port: u64,
    db_directory: PathBuf,
    command: MithrilCommand,
    process: Option<Child>,
}

impl Aggregator {
    pub fn new(aggregator_config: &AggregatorConfig) -> StdResult<Self> {
        let magic_id = DEVNET_MAGIC_ID.to_string();
        let server_port_parameter = aggregator_config.server_port.to_string();
        let era_reader_adapter_params = format!(
            r#"{{"markers": [{{"name": "{}", "epoch": 0}}]}}"#,
            aggregator_config.mithril_era
        );
        let signed_entity_types = aggregator_config.signed_entity_types.join(",");
        let env = HashMap::from([
            ("NETWORK", "devnet"),
            ("RUN_INTERVAL", "200"),
            ("SERVER_IP", "0.0.0.0"),
            ("SERVER_PORT", &server_port_parameter),
            ("URL_SNAPSHOT_MANIFEST", ""),
            ("SNAPSHOT_STORE_TYPE", "local"),
            ("SNAPSHOT_UPLOADER_TYPE", "local"),
            ("NETWORK_MAGIC", &magic_id),
            ("DATA_STORES_DIRECTORY", "./stores/aggregator"),
            (
                "CARDANO_NODE_SOCKET_PATH",
                aggregator_config.bft_node.socket_path.to_str().unwrap(),
            ),
            ("STORE_RETENTION_LIMIT", "10"),
            ("CARDANO_CLI_PATH", aggregator_config.cardano_cli_path.to_str().unwrap()),
            ("GENESIS_VERIFICATION_KEY", "5b33322c3235332c3138362c3230312c3137372c31312c3131372c3133352c3138372c3136372c3138312c3138382c32322c35392c3230362c3130352c3233312c3135302c3231352c33302c37382c3231322c37362c31362c3235322c3138302c37322c3133342c3133372c3234372c3136312c36385d"),
            ("GENESIS_SECRET_KEY", "5b3131382c3138342c3232342c3137332c3136302c3234312c36312c3134342c36342c39332c3130362c3232392c38332c3133342c3138392c34302c3138392c3231302c32352c3138342c3136302c3134312c3233372c32362c3136382c35342c3233392c3230342c3133392c3131392c31332c3139395d"),
            ("ERA_READER_ADAPTER_TYPE", "dummy"),
            ("ERA_READER_ADAPTER_PARAMS", &era_reader_adapter_params),
            ("SIGNED_ENTITY_TYPES", &signed_entity_types),
            ("CARDANO_NODE_VERSION", "8.1.2"),
            ("CHAIN_OBSERVER_TYPE", aggregator_config.chain_observer_type),
        ]);
        let args = vec![
            "--db-directory",
            aggregator_config.bft_node.db_path.to_str().unwrap(),
            "-vvv",
        ];

        let command = MithrilCommand::new(
            "mithril-aggregator",
            aggregator_config.work_dir,
            aggregator_config.bin_dir,
            env,
            &args,
        )?;

        Ok(Self {
            server_port: aggregator_config.server_port,
            db_directory: aggregator_config.bft_node.db_path.clone(),
            command,
            process: None,
        })
    }

    pub fn copy_configuration(other: &Aggregator) -> Self {
        Self {
            server_port: other.server_port,
            db_directory: other.db_directory.clone(),
            command: other.command.clone(),
            process: None,
        }
    }

    pub fn endpoint(&self) -> String {
        format!("http://localhost:{}/aggregator", &self.server_port)
    }

    pub fn db_directory(&self) -> &Path {
        &self.db_directory
    }

    pub fn serve(&mut self) -> StdResult<()> {
        self.process = Some(self.command.start(&["serve".to_string()])?);
        Ok(())
    }

    pub async fn bootstrap_genesis(&mut self) -> StdResult<()> {
        let exit_status = self
            .command
            .start(&["genesis".to_string(), "bootstrap".to_string()])?
            .wait()
            .await
            .with_context(|| "`mithril-aggregator genesis bootstrap` crashed")?;

        if exit_status.success() {
            Ok(())
        } else {
            Err(match exit_status.code() {
                Some(c) => {
                    anyhow!("`mithril-aggregator genesis bootstrap` exited with code: {c}")
                }
                None => {
                    anyhow!("`mithril-aggregator genesis bootstrap` was terminated with a signal")
                }
            })
        }
    }

    pub async fn stop(&mut self) -> StdResult<()> {
        if let Some(process) = self.process.as_mut() {
            process
                .kill()
                .await
                .with_context(|| "Could not kill aggregator")?;
        }
        Ok(())
    }

    pub fn set_protocol_parameters(&mut self, protocol_parameters: &entities::ProtocolParameters) {
        self.command.set_env_var(
            "PROTOCOL_PARAMETERS__K",
            &format!("{}", protocol_parameters.k),
        );
        self.command.set_env_var(
            "PROTOCOL_PARAMETERS__M",
            &format!("{}", protocol_parameters.m),
        );
        self.command.set_env_var(
            "PROTOCOL_PARAMETERS__PHI_F",
            &format!("{}", protocol_parameters.phi_f),
        );
    }

    pub fn set_mock_cardano_cli_file_path(
        &mut self,
        stake_distribution_file: &Path,
        epoch_file_path: &Path,
    ) {
        self.command.set_env_var(
            "MOCK_STAKE_DISTRIBUTION_FILE",
            stake_distribution_file.to_str().unwrap(),
        );
        self.command
            .set_env_var("MOCK_EPOCH_FILE", epoch_file_path.to_str().unwrap());
    }

    /// Change the run interval of the aggregator state machine (default: 400ms)
    pub fn change_run_interval(&mut self, interval: Duration) {
        self.command
            .set_env_var("RUN_INTERVAL", &format!("{}", interval.as_millis()))
    }

    pub async fn tail_logs(&self, number_of_line: u64) -> StdResult<()> {
        self.command.tail_logs(None, number_of_line).await
    }
}
