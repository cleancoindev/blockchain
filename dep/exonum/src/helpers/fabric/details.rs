// Copyright 2017 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(missing_debug_implementations)]

//! This module implement all core commands.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use toml::Value;

use super::internal::{CollectedCommand, Command, Feedback};
use super::keys;
use super::shared::{
    AbstractConfig, CommonConfigTemplate, NodePrivateConfig, NodePublicConfig, SharedConfig,
};
use super::DEFAULT_EXONUM_LISTEN_PORT;
use super::{Argument, CommandName, Context};
use blockchain::config::ValidatorKeys;
use blockchain::GenesisConfig;
use crypto;
use helpers::config::ConfigFile;
use helpers::generate_testnet_config;
use node::{NodeApiConfig, NodeConfig};
use storage::Database;

const DATABASE_PATH: &str = "DATABASE_PATH";

/// Run command.
pub struct Run;

impl Run {
    /// Returns the name of the `Run` command.
    pub fn name() -> CommandName {
        "run"
    }

    /// Returns created database instance.
    pub fn db_helper(ctx: &Context) -> Box<Database> {
        use storage::{RocksDB, RocksDBOptions};

        let path = ctx
            .arg::<String>(DATABASE_PATH)
            .expect(&format!("{} not found.", DATABASE_PATH));
        let mut options = RocksDBOptions::default();
        options.create_if_missing(true);
        Box::new(RocksDB::open(Path::new(&path), &options).unwrap())
    }

    fn node_config(ctx: &Context) -> NodeConfig {
        let path = ctx
            .arg::<String>("NODE_CONFIG_PATH")
            .expect("NODE_CONFIG_PATH not found.");
        ConfigFile::load(path).unwrap()
    }

    fn public_api_address(ctx: &Context) -> Option<SocketAddr> {
        ctx.arg("PUBLIC_API_ADDRESS").ok()
    }

    fn private_api_address(ctx: &Context) -> Option<SocketAddr> {
        ctx.arg("PRIVATE_API_ADDRESS").ok()
    }
}

impl Command for Run {
    fn args(&self) -> Vec<Argument> {
        vec![
            Argument::new_named(
                "NODE_CONFIG_PATH",
                true,
                "Path to node configuration file.",
                "c",
                "node-config",
                false,
            ),
            Argument::new_named(
                DATABASE_PATH,
                true,
                "Use database with the given path.",
                "d",
                "db-path",
                false,
            ),
            Argument::new_named(
                "PUBLIC_API_ADDRESS",
                false,
                "Listen address for public api.",
                None,
                "public-api-address",
                false,
            ),
            Argument::new_named(
                "PRIVATE_API_ADDRESS",
                false,
                "Listen address for private api.",
                None,
                "private-api-address",
                false,
            ),
        ]
    }

    fn name(&self) -> CommandName {
        Self::name()
    }

    fn about(&self) -> &str {
        "Run application"
    }

    fn execute(
        &self,
        _commands: &HashMap<CommandName, CollectedCommand>,
        mut context: Context,
        exts: &Fn(Context) -> Context,
    ) -> Feedback {
        let config = Self::node_config(&context);
        let public_addr = Self::public_api_address(&context);
        let private_addr = Self::private_api_address(&context);

        context.set(keys::NODE_CONFIG, config);
        let mut new_context = exts(context);
        let mut config = new_context
            .get(keys::NODE_CONFIG)
            .expect("cant load node_config");
        // Override api options
        if let Some(public_addr) = public_addr {
            config.api.public_api_address = Some(public_addr);
        }

        if let Some(private_api_address) = private_addr {
            config.api.private_api_address = Some(private_api_address);
        }

        new_context.set(keys::NODE_CONFIG, config);

        Feedback::RunNode(new_context)
    }
}

/// Command for running service in dev mode.
pub struct RunDev;

impl RunDev {
    /// Returns the name of the `Run` command.
    pub fn name() -> CommandName {
        "run-dev"
    }

    fn artifacts_directory(ctx: &Context) -> PathBuf {
        let directory = ctx
            .arg::<String>("ARTIFACTS_DIR")
            .unwrap_or_else(|_| ".exonum".into());
        PathBuf::from(&directory)
    }

    fn artifacts_path(inner_path: &str, ctx: &Context) -> String {
        let mut path = Self::artifacts_directory(ctx);
        path.push(inner_path);
        path.to_str().expect("Expected correct path").into()
    }

    fn generate_config(commands: &HashMap<CommandName, CollectedCommand>, ctx: &Context) -> String {
        let common_config_path = Self::artifacts_path("common.toml", ctx);
        let peer_addr = "127.0.0.1";
        let pub_config_path = Self::artifacts_path("public.toml", ctx);
        let sec_config_path = Self::artifacts_path("secret.toml", ctx);
        let output_config_path = Self::artifacts_path("output.toml", ctx);

        let mut common_config_ctx = ctx.clone();
        common_config_ctx.set_arg("COMMON_CONFIG", common_config_path.clone());
        let common_config_command = commands
            .get(GenerateCommonConfig::name())
            .expect("Expected GenerateCommonConfig in the commands list.");
        common_config_command.execute(commands, common_config_ctx);

        let mut node_config_ctx = ctx.clone();
        node_config_ctx.set_arg("COMMON_CONFIG", common_config_path.clone());
        node_config_ctx.set_arg("PUB_CONFIG", pub_config_path.clone());
        node_config_ctx.set_arg("SEC_CONFIG", sec_config_path.clone());
        node_config_ctx.set_arg("PEER_ADDR", peer_addr.into());
        let node_config_command = commands
            .get(GenerateNodeConfig::name())
            .expect("Expected GenerateNodeConfig in the commands list.");
        node_config_command.execute(commands, node_config_ctx);

        let mut finalize_ctx = ctx.clone();
        finalize_ctx.set_arg_multiple("PUBLIC_CONFIGS", vec![pub_config_path.clone()]);
        finalize_ctx.set_arg("SECRET_CONFIG", sec_config_path.clone());
        finalize_ctx.set_arg("OUTPUT_CONFIG_PATH", output_config_path.clone());
        let finalize_command = commands
            .get(Finalize::name())
            .expect("Expected Finalize in the commands list.");
        finalize_command.execute(commands, finalize_ctx);

        output_config_path
    }

    fn cleanup(ctx: &Context) {
        let database_dir_path = ctx
            .arg::<String>(DATABASE_PATH)
            .expect("Expected DATABASE_PATH being set.");
        let database_dir = Path::new(&database_dir_path);
        if database_dir.exists() {
            fs::remove_dir_all(Self::artifacts_directory(ctx))
                .expect("Expected DATABASE_PATH folder being removable.");
        }
    }
}

impl Command for RunDev {
    fn args(&self) -> Vec<Argument> {
        vec![Argument::new_named(
            "ARTIFACTS_DIR",
            false,
            "The path where configuration and db files will be generated.",
            "a",
            "artifacts-dir",
            false,
        )]
    }

    fn name(&self) -> CommandName {
        Self::name()
    }

    fn about(&self) -> &str {
        "Run application in development mode (generate configuration and db files automatically)"
    }

    fn execute(
        &self,
        commands: &HashMap<CommandName, CollectedCommand>,
        mut context: Context,
        exts: &Fn(Context) -> Context,
    ) -> Feedback {
        let db_path = Self::artifacts_path("db", &context);
        context.set_arg(DATABASE_PATH, db_path);
        Self::cleanup(&context);

        let node_config_path = Self::generate_config(commands, &context);
        context.set_arg("NODE_CONFIG_PATH", node_config_path);

        let new_context = exts(context);
        commands
            .get(Run::name())
            .expect("Expected Run in the commands list.")
            .execute(commands, new_context.clone())
    }
}

/// Command for the template generation.
pub struct GenerateCommonConfig;

impl GenerateCommonConfig {
    /// Returns the name of the `GenerateCommonConfig` command.
    pub fn name() -> CommandName {
        "generate-template"
    }
}

impl Command for GenerateCommonConfig {
    fn args(&self) -> Vec<Argument> {
        vec![Argument::new_positional(
            "COMMON_CONFIG",
            true,
            "Path to common config.",
        )]
    }

    fn name(&self) -> CommandName {
        Self::name()
    }

    fn about(&self) -> &str {
        "Generate basic config template."
    }

    fn execute(
        &self,
        _commands: &HashMap<CommandName, CollectedCommand>,
        mut context: Context,
        exts: &Fn(Context) -> Context,
    ) -> Feedback {
        let template_path = context
            .arg::<String>("COMMON_CONFIG")
            .expect("COMMON_CONFIG not found");

        context.set(keys::SERVICES_CONFIG, AbstractConfig::default());
        let new_context = exts(context);
        let services_config = new_context.get(keys::SERVICES_CONFIG).unwrap_or_default();

        let template = CommonConfigTemplate {
            services_config,
            ..CommonConfigTemplate::default()
        };

        ConfigFile::save(&template, template_path).expect("Could not write template file.");
        Feedback::None
    }
}

/// Command for the node configuration generation.
pub struct GenerateNodeConfig;

impl GenerateNodeConfig {
    /// Returns the name of the `GenerateNodeConfig` command.
    pub fn name() -> CommandName {
        "generate-config"
    }

    fn addr(context: &Context) -> (SocketAddr, SocketAddr) {
        let addr = context.arg::<String>("PEER_ADDR").unwrap_or_default();

        let mut addr_parts = addr.split(':');
        let ip = addr_parts.next().expect("Expected ip address");
        if ip.len() < 8 {
            panic!("Expected ip address in PEER_ADDR.")
        }
        let port = addr_parts.next().map_or(DEFAULT_EXONUM_LISTEN_PORT, |s| {
            s.parse().expect("could not parse port")
        });
        let external_addr = format!("{}:{}", ip, port);
        let listen_addr = format!("0.0.0.0:{}", port);
        (external_addr.parse().unwrap(), listen_addr.parse().unwrap())
    }
}

impl Command for GenerateNodeConfig {
    fn args(&self) -> Vec<Argument> {
        vec![
            Argument::new_positional("COMMON_CONFIG", true, "Path to common config."),
            Argument::new_positional("PUB_CONFIG", true, "Path where save public config."),
            Argument::new_positional("SEC_CONFIG", true, "Path where save private config."),
            Argument::new_named(
                "PEER_ADDR",
                true,
                "Remote peer address",
                "a",
                "peer-addr",
                false,
            ),
        ]
    }

    fn name(&self) -> CommandName {
        Self::name()
    }

    fn about(&self) -> &str {
        "Generate node secret and public configs."
    }

    fn execute(
        &self,
        _commands: &HashMap<CommandName, CollectedCommand>,
        mut context: Context,
        exts: &Fn(Context) -> Context,
    ) -> Feedback {
        let common_config_path = context
            .arg::<String>("COMMON_CONFIG")
            .expect("expected common config path");
        let pub_config_path = context
            .arg::<String>("PUB_CONFIG")
            .expect("expected public config path");
        let priv_config_path = context
            .arg::<String>("SEC_CONFIG")
            .expect("expected secret config path");

        let addr = Self::addr(&context);
        let common: CommonConfigTemplate =
            ConfigFile::load(&common_config_path).expect("Could not load common config");
        context.set(keys::COMMON_CONFIG, common.clone());
        context.set(
            keys::SERVICES_PUBLIC_CONFIGS,
            BTreeMap::<String, Value>::default(),
        );
        context.set(
            keys::SERVICES_SECRET_CONFIGS,
            BTreeMap::<String, Value>::default(),
        );
        let new_context = exts(context);
        let services_public_configs = new_context.get(keys::SERVICES_PUBLIC_CONFIGS).unwrap();
        let services_secret_configs = new_context.get(keys::SERVICES_SECRET_CONFIGS);

        let (consensus_public_key, consensus_secret_key) = crypto::gen_keypair();
        let (service_public_key, service_secret_key) = crypto::gen_keypair();

        let validator_keys = ValidatorKeys {
            consensus_key: consensus_public_key,
            service_key: service_public_key,
        };
        let node_pub_config = NodePublicConfig {
            addr: addr.0,
            validator_keys,
            services_public_configs,
        };
        let shared_config = SharedConfig {
            node: node_pub_config,
            common: common,
        };
        // Save public config separately.
        ConfigFile::save(&shared_config, &pub_config_path)
            .expect("Could not write public config file.");

        let priv_config = NodePrivateConfig {
            listen_addr: addr.1,
            consensus_public_key,
            consensus_secret_key,
            service_public_key,
            service_secret_key,
            services_secret_configs: services_secret_configs
                .expect("services_secret_configs not found after exts call"),
        };

        ConfigFile::save(&priv_config, priv_config_path)
            .expect("Could not write secret config file.");
        Feedback::None
    }
}

/// Finalize command.
pub struct Finalize;

impl Finalize {
    /// Returns the name of the `Finalize` command.
    pub fn name() -> CommandName {
        "finalize"
    }

    /// Returns `GenesisConfig` from the template.
    fn genesis_from_template(
        template: CommonConfigTemplate,
        configs: &[NodePublicConfig],
    ) -> GenesisConfig {
        GenesisConfig::new_with_consensus(
            template.consensus_config,
            configs.iter().map(|c| c.validator_keys),
        )
    }

    fn reduce_configs(
        public_configs: Vec<SharedConfig>,
        our_config: &NodePrivateConfig,
    ) -> (
        CommonConfigTemplate,
        Vec<NodePublicConfig>,
        Option<NodePublicConfig>,
    ) {
        let mut map = BTreeMap::new();
        let mut config_iter = public_configs.into_iter();
        let first = config_iter
            .next()
            .expect("Expected at least one config in PUBLIC_CONFIGS");
        let common = first.common;
        map.insert(first.node.validator_keys.consensus_key, first.node);

        for config in config_iter {
            if common != config.common {
                panic!("Found config with different common part.");
            };
            if map
                .insert(config.node.validator_keys.consensus_key, config.node)
                .is_some()
            {
                panic!("Found duplicate consensus keys in PUBLIC_CONFIGS");
            }
        }
        (
            common,
            map.iter().map(|(_, c)| c.clone()).collect(),
            map.get(&our_config.consensus_public_key).cloned(),
        )
    }
}

impl Command for Finalize {
    fn args(&self) -> Vec<Argument> {
        vec![
            Argument::new_named(
                "PUBLIC_CONFIGS",
                true,
                "Path to validators public configs",
                "p",
                "public-configs",
                true,
            ),
            Argument::new_named(
                "PUBLIC_API_ADDRESS",
                false,
                "Listen address for public api.",
                None,
                "public-api-address",
                false,
            ),
            Argument::new_named(
                "PRIVATE_API_ADDRESS",
                false,
                "Listen address for private api.",
                None,
                "private-api-address",
                false,
            ),
            Argument::new_positional("SECRET_CONFIG", true, "Path to our secret config."),
            Argument::new_positional("OUTPUT_CONFIG_PATH", true, "Path to output node config."),
        ]
    }

    fn name(&self) -> CommandName {
        Self::name()
    }

    fn about(&self) -> &str {
        "Collect public and secret configs into node config."
    }

    fn execute(
        &self,
        _commands: &HashMap<CommandName, CollectedCommand>,
        mut context: Context,
        exts: &Fn(Context) -> Context,
    ) -> Feedback {
        let public_configs_path = context
            .arg_multiple::<String>("PUBLIC_CONFIGS")
            .expect("keychain path not found");
        let secret_config_path = context
            .arg::<String>("SECRET_CONFIG")
            .expect("config path not found");
        let output_config_path = context
            .arg::<String>("OUTPUT_CONFIG_PATH")
            .expect("config path not found");

        let public_addr = Run::public_api_address(&context);
        let private_addr = Run::private_api_address(&context);

        let secret_config: NodePrivateConfig =
            ConfigFile::load(secret_config_path).expect("Failed to load key config.");
        let public_configs: Vec<SharedConfig> = public_configs_path
            .into_iter()
            .map(|path| ConfigFile::load(path).expect("Failed to load validator public config."))
            .collect();
        let (common, list, our) = Self::reduce_configs(public_configs, &secret_config);

        context.set(keys::AUDITOR_MODE, our.is_none());

        let peers = list.iter().map(|c| c.addr).collect();

        let genesis = Self::genesis_from_template(common.clone(), &list);

        let config = {
            NodeConfig {
                listen_address: secret_config.listen_addr,
                external_address: our.map(|o| o.addr),
                network: Default::default(),
                whitelist: Default::default(),
                peers: peers,
                consensus_public_key: secret_config.consensus_public_key,
                consensus_secret_key: secret_config.consensus_secret_key,
                service_public_key: secret_config.service_public_key,
                service_secret_key: secret_config.service_secret_key,
                genesis: genesis,
                api: NodeApiConfig {
                    public_api_address: public_addr,
                    private_api_address: private_addr,
                    ..Default::default()
                },
                mempool: Default::default(),
                services_configs: Default::default(),
            }
        };

        context.set(keys::PUBLIC_CONFIG_LIST, list);
        context.set(keys::NODE_CONFIG, config);
        context.set(keys::COMMON_CONFIG, common);
        context.set(
            keys::SERVICES_SECRET_CONFIGS,
            secret_config.services_secret_configs,
        );

        let new_context = exts(context);

        let config = new_context
            .get(keys::NODE_CONFIG)
            .expect("Could not create config from template, services return error");
        ConfigFile::save(&config, output_config_path).expect("Could not write config file.");

        Feedback::None
    }
}

/// Command for the testnet generation.
pub struct GenerateTestnet;

impl GenerateTestnet {
    /// Returns the name of the `GenerateTestnet` command.
    pub fn name() -> CommandName {
        "generate-testnet"
    }
}

impl Command for GenerateTestnet {
    fn args(&self) -> Vec<Argument> {
        vec![
            Argument::new_named(
                "OUTPUT_DIR",
                true,
                "Path to directory where save configs.",
                "o",
                "output_dir",
                false,
            ),
            Argument::new_named(
                "START_PORT",
                false,
                "Port number started from which should validators listen.",
                "p",
                "start",
                false,
            ),
            Argument::new_positional("COUNT", true, "Count of validators in testnet."),
        ]
    }

    fn name(&self) -> CommandName {
        Self::name()
    }

    fn about(&self) -> &str {
        "Generates genesis configuration for testnet"
    }

    fn execute(
        &self,
        _commands: &HashMap<CommandName, CollectedCommand>,
        mut context: Context,
        exts: &Fn(Context) -> Context,
    ) -> Feedback {
        let dir = context.arg::<String>("OUTPUT_DIR").expect("output dir");
        let count: u8 = context.arg("COUNT").expect("count as int");
        let start_port = context
            .arg::<u16>("START_PORT")
            .unwrap_or(DEFAULT_EXONUM_LISTEN_PORT);

        if count == 0 {
            panic!("Can't generate testnet with zero nodes count.");
        }

        let dir = Path::new(&dir);
        let dir = dir.join("validators");
        if !dir.exists() {
            fs::create_dir_all(&dir).unwrap();
        }

        let configs = generate_testnet_config(count, start_port);
        context.set(keys::CONFIGS, configs);
        let new_context = exts(context);
        let configs = new_context
            .get(keys::CONFIGS)
            .expect("Couldn't read testnet configs after exts call.");

        for (idx, cfg) in configs.into_iter().enumerate() {
            let file_name = format!("{}.toml", idx);
            ConfigFile::save(&cfg, &dir.join(file_name)).unwrap();
        }

        Feedback::None
    }
}
