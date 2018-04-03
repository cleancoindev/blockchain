use exonum::crypto;
use exonum::crypto::{PublicKey, Signature};
use exonum::blockchain::Transaction;
use exonum::storage::Fork;
use exonum::messages::Message;
use serde_json;

use currency::{SERVICE_ID, Service};
use currency::assets::TradeAsset;
use currency::transactions::components::Intermediary;
use currency::transactions::components::{FeeStrategy, ThirdPartyFees};
use currency::error::Error;
use currency::status;
use currency::wallet;
use currency::configuration::Configuration;

/// Transaction ID.
pub const TRADE_INTERMEDIARY_ID: u16 = 502;

encoding_struct! {
    struct TradeOfferIntermediary {
        const SIZE = 81;

        field intermediary: Intermediary       [00 => 08]
        field buyer:        &PublicKey         [08 => 40]
        field seller:       &PublicKey         [40 => 72]
        field assets:       Vec<TradeAsset>    [72 => 80]

        field fee_strategy: u8                 [80 => 81]
    }
}

message! {
    /// `trade_intermediary` transaction.
    struct TradeIntermediary {
        const TYPE = SERVICE_ID;
        const ID = TRADE_INTERMEDIARY_ID;
        const SIZE = 152;

        field offer:                  TradeOfferIntermediary     [00 => 08]
        field seed:                   u64                        [08 => 16]
        field seller_signature:       &Signature                 [16 => 80]
        field intermediary_signature: &Signature                 [80 => 144]
        field data_info:              &str                       [144 => 152]
    }
}

impl TradeIntermediary {
    /// Raw bytes of the offer.
    pub fn offer_raw(&self) -> Vec<u8> {
        self.offer().raw
    }

    fn process(&self, view: &mut Fork) -> Result<(), Error> {
        info!("Processing tx: {:?}", self);

        let genesis_fee = Configuration::extract(view).fees().trade();

        let offer = self.offer();

        let fee_strategy =
            FeeStrategy::try_from(offer.fee_strategy()).expect("fee strategy must be valid");

        let total = offer.assets()
            .iter()
            .map(|asset| {asset.amount() * asset.price()})
            .sum::<u64>();

        let mut genesis = wallet::Schema(&*view).fetch(&Service::genesis_wallet());

        // Collect the blockchain fee. Execution shall not continue if this fails.
        match fee_strategy {
            FeeStrategy::Recipient => {
                let mut buyer = wallet::Schema(&*view).fetch(offer.buyer());

                wallet::move_coins(&mut buyer, &mut genesis, genesis_fee)?;

                wallet::Schema(&mut *view).store(offer.buyer(), buyer);
            }
            FeeStrategy::Sender => {
                let mut seller = wallet::Schema(&*view).fetch(offer.seller());

                wallet::move_coins(&mut seller, &mut genesis, genesis_fee)?;

                wallet::Schema(&mut *view).store(offer.seller(), seller);
            }
            FeeStrategy::RecipientAndSender => {
                let mut buyer = wallet::Schema(&*view).fetch(offer.buyer());
                let mut seller = wallet::Schema(&*view).fetch(offer.seller());

                wallet::move_coins(&mut seller, &mut genesis, genesis_fee / 2)?;
                wallet::move_coins(&mut buyer, &mut genesis, genesis_fee / 2)?;

                wallet::Schema(&mut *view).store(offer.seller(), seller);
                wallet::Schema(&mut *view).store(offer.buyer(), buyer);
            }
            FeeStrategy::Intermediary => {
                let mut intermediary = wallet::Schema(&*view).fetch(offer.intermediary().wallet());

                wallet::move_coins(&mut intermediary, &mut genesis, genesis_fee)?;

                wallet::Schema(&mut *view).store(offer.intermediary().wallet(), intermediary);
            }
        }

        wallet::Schema(&mut *view).store(&Service::genesis_wallet(), genesis);

	let mut fees = ThirdPartyFees::new_trade(&*view,&offer.assets())?;
        fees.add_fee(
            offer.intermediary().wallet(),
            offer.intermediary().commission(),
        );
        let mut wallet_buyer = wallet::Schema(&*view).fetch(self.offer().buyer());
        let mut wallet_seller = wallet::Schema(&*view).fetch(self.offer().seller());

        wallet::move_coins(&mut wallet_buyer, &mut wallet_seller, total)
            .or_else(|e| {
                wallet::Schema(&mut *view).store(&offer.seller(), wallet_seller.clone());
                wallet::Schema(&mut *view).store(&offer.buyer(), wallet_buyer.clone());

                Err(e)
            })
            .and_then(|_| {
                wallet::Schema(&mut *view).store(&offer.seller(), wallet_seller);
                wallet::Schema(&mut *view).store(&offer.buyer(), wallet_buyer);

                let mut updated_wallets = match fee_strategy {
                    FeeStrategy::Recipient => fees.collect(view, offer.buyer())?,
                    FeeStrategy::Sender => fees.collect(view, offer.seller())?,
                    FeeStrategy::RecipientAndSender => {
                        fees.collect2(view, offer.seller(), offer.buyer())?
                    },
                    FeeStrategy::Intermediary => fees.collect(view, offer.intermediary().wallet())?,
                };

                let mut wallet_seller = updated_wallets
                    .remove(&offer.seller())
                    .unwrap_or_else(|| wallet::Schema(&*view).fetch(&offer.seller()));
                let mut wallet_buyer = updated_wallets
                    .remove(&offer.buyer())
                    .unwrap_or_else(|| wallet::Schema(&*view).fetch(&offer.buyer()));
                let assets = offer.assets()
                    .into_iter()
                    .map(|a| a.to_bundle())
                    .collect::<Vec<_>>();

                wallet::move_assets(&mut wallet_seller, &mut wallet_buyer, &assets)?;

                updated_wallets.insert(*offer.seller(), wallet_seller);
                updated_wallets.insert(*offer.buyer(), wallet_buyer);

                // Save changes to the database.
                for (key, wallet) in updated_wallets {
                    wallet::Schema(&mut *view).store(&key, wallet);
                }

                Ok(())
            })?;

        Ok(())
    }
}

impl Transaction for TradeIntermediary {
    fn verify(&self) -> bool {
        let offer = self.offer();

        let wallets_ok = offer.seller() != offer.buyer()
            && offer.intermediary().wallet() != offer.seller()
            && offer.intermediary().wallet() != offer.buyer();
        let fee_strategy_ok = FeeStrategy::try_from(offer.fee_strategy()).is_some();

        if cfg!(fuzzing) {
            return wallets_ok && fee_strategy_ok;
        }

        let buyer_ok = self.verify_signature(offer.buyer());

        let seller_ok = crypto::verify(
            self.seller_signature(),
            &offer.raw,
            offer.seller()
        );
        let intermediary_ok = crypto::verify(
            self.intermediary_signature(),
            &offer.raw,
            offer.intermediary().wallet(),
        );

        wallets_ok && fee_strategy_ok && buyer_ok && seller_ok && intermediary_ok
    }

    fn execute(&self, view: &mut Fork) {
        let result = self.process(view);
        status::Schema(view).store(self.hash(), result);
    }

    fn info(&self) -> serde_json::Value {
        json!({})
    }
}