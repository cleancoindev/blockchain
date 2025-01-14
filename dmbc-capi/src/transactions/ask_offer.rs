use crypto::PublicKey;

use assets::TradeAsset;
use transactions::components::service::SERVICE_ID;

use error::{Error, ErrorKind};

/// Transaction ID.
pub const ASK_OFFER_ID: u16 = 701;

evo_message! {
    /// `AskOffer` transaction.
    struct AskOffer {
        const TYPE = SERVICE_ID;
        const ID = ASK_OFFER_ID;

        pub_key:      &PublicKey,
        asset:        TradeAsset,
        seed:         u64,
        memo:         &str,
    }
}

#[derive(Clone, Debug)]
pub struct AskOfferWrapper {
    pub_key:      PublicKey,
    asset:        TradeAsset,
    seed:         u64,
    memo:    String,
}

impl AskOfferWrapper {
    pub fn new(
        pub_key: &PublicKey,
        asset: TradeAsset,
        seed: u64,
        memo: &str
    ) -> Self {
        AskOfferWrapper {
            pub_key: *pub_key,
            asset: asset.clone(),
            seed: seed,
            memo: memo.to_string()
        }
    }

    pub fn from_ptr<'a>(
        builder: *mut AskOfferWrapper,
    ) -> Result<&'a mut AskOfferWrapper, Error> {
        if builder.is_null() {
            return Err(Error::new(ErrorKind::Text(
                "Offer isn't initialized".to_string(),
            )));
        }
        Ok(unsafe { &mut *builder })
    }

    pub fn unwrap(&self) -> AskOffer {
        AskOffer::new(
            &self.pub_key,
            self.asset.clone(),
            self.seed,
            self.memo.as_str()
        )
    }
}