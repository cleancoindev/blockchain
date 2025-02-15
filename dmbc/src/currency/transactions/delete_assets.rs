use std::collections::HashMap;

use exonum::blockchain::Transaction;
use exonum::crypto::PublicKey;
use exonum::messages::Message;
use exonum::storage::Fork;
use prometheus::{Histogram, IntCounter};

use currency::assets;
use currency::assets::AssetBundle;
use currency::error::Error;
use currency::service::CONFIGURATION;
use currency::status;
use currency::transactions::components::FeesCalculator;
use currency::transactions::components::permissions;
use currency::wallet;
use currency::SERVICE_ID;

/// Transaction ID.
pub const DELETE_ASSETS_ID: u16 = 400;

message! {
    /// `delete_assets` transaction.
    struct DeleteAssets {
        const TYPE = SERVICE_ID;
        const ID = DELETE_ASSETS_ID;

        pub_key:     &PublicKey,
        assets:      Vec<AssetBundle>,
        seed:        u64,
    }
}

impl FeesCalculator for DeleteAssets {
    fn calculate_fees(&self, _view: &mut Fork) -> Result<HashMap<PublicKey, u64>, Error> {
        let genesis_fees = CONFIGURATION.read().unwrap().fees();
        let tx_fee = genesis_fees.delete_assets();

        let mut fees_table = HashMap::new();
        if genesis_fees.recipient() != self.pub_key() {
            fees_table.insert(*self.pub_key(), tx_fee);
        }
        Ok(fees_table)
    }
}

impl DeleteAssets {
    fn process(&self, view: &mut Fork) -> Result<(), Error> {
        info!("Processing tx: {:?}", self);

        let genesis_fees = CONFIGURATION.read().unwrap().fees();

        let genesis_pub = genesis_fees.recipient();
        let owner_pub = self.pub_key();

        let mut genesis = wallet::Schema(&*view).fetch(&genesis_pub);
        let mut owner = wallet::Schema(&*view).fetch(&owner_pub);

        wallet::move_coins(&mut owner, &mut genesis, genesis_fees.delete_assets())?;

        wallet::Schema(&mut *view).store(&genesis_pub, genesis);
        wallet::Schema(&mut *view).store(&owner_pub, owner.clone());

        let mut infos = HashMap::new();

        for asset in self.assets() {
            let info = match assets::Schema(&*view).fetch(&asset.id()) {
                Some(info) => info,
                None => return Err(Error::AssetNotFound),
            };
            let mut entry = infos.remove(&asset.id()).unwrap_or(info);
            let entry = entry.decrease(asset.amount())?;
            infos.insert(asset.id(), entry);
        }

        owner.remove_assets(self.assets())?;

        wallet::Schema(&mut *view).store(owner_pub, owner);

        for (id, info) in infos {
            assets::Schema(&mut *view).store(&id, info);
        }

        Ok(())
    }
}

lazy_static! {
    static ref VERIFY_COUNT: IntCounter = register_int_counter!(
        "dmbc_transaction_delete_assets_verify_count",
        "Times .verify() was called on a transaction."
    ).unwrap();
    static ref VERIFY_SUCCESS_COUNT: IntCounter = register_int_counter!(
        "dmbc_transaction_delete_assets_verify_success_count",
        "Times verification was successfull on a transaction."
    ).unwrap();
    static ref EXECUTE_COUNT: IntCounter = register_int_counter!(
        "dmbc_transaction_delete_assets_execute_count",
        "Transactions executed."
    ).unwrap();
    static ref EXECUTE_SUCCESS_COUNT: IntCounter = register_int_counter!(
        "dmbc_transaction_delete_assets_execute_success_count",
        "Times transaction execution reported a success."
    ).unwrap();
    static ref EXECUTE_FINISH_COUNT: IntCounter = register_int_counter!(
        "dmbc_transaction_delete_assets_execute_finish_count",
        "Times transaction has finished executing without panicking."
    ).unwrap();
    static ref EXECUTE_DURATION: Histogram = register_histogram!(
        "dmbc_transaction_delete_assets_execute_duration_seconds",
        "Duration of transaction execution."
    ).unwrap();
}

impl Transaction for DeleteAssets {
    fn verify(&self) -> bool {
        VERIFY_COUNT.inc();

        if cfg!(fuzzing) {
            return true;
        }

        if !permissions::is_authorized(DELETE_ASSETS_ID, vec![self.pub_key()]) {
            return false;
        }

        if self.verify_signature(self.pub_key()) {
            VERIFY_SUCCESS_COUNT.inc();
            true
        } else {
            false
        }
    }

    fn execute(&self, view: &mut Fork) {
        EXECUTE_COUNT.inc();
        let timer = EXECUTE_DURATION.start_timer();

        let result = self.process(view);

        if let &Ok(_) = &result {
            EXECUTE_SUCCESS_COUNT.inc();
        }

        status::Schema(view).store(self.hash(), result);

        timer.observe_duration();
        EXECUTE_FINISH_COUNT.inc();
    }
}
