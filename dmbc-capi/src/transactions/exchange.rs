use exonum::crypto::{PublicKey, SecretKey, Signature};

use assets::AssetBundle;
use transactions::components::service::SERVICE_ID;

use error::{Error, ErrorKind};

/// Transaction ID.
pub const EXCHANGE_ID: u16 = 601;

encoding_struct! {
    struct ExchangeOffer {
        sender:           &PublicKey,
        sender_assets:    Vec<AssetBundle>,
        sender_value:     u64,

        recipient:        &PublicKey,
        recipient_assets: Vec<AssetBundle>,

        fee_strategy:     u8,
    }
}

#[derive(Debug, Clone)]
pub struct ExchangeOfferWrapper {
    sender: PublicKey,
    sender_assets: Vec<AssetBundle>,
    sender_value: u64,

    recipient: PublicKey,
    recipient_assets: Vec<AssetBundle>,

    fee_strategy: u8,
}

impl ExchangeOfferWrapper {
    pub fn new(
        sender: &PublicKey,
        sender_value: u64,
        recipient: &PublicKey,
        fee_strategy: u8,
    ) -> Self {
        ExchangeOfferWrapper {
            sender: *sender,
            sender_assets: Vec::new(),
            sender_value: sender_value,

            recipient: *recipient,
            recipient_assets: Vec::new(),
            fee_strategy: fee_strategy,
        }
    }

    pub fn from_ptr<'a>(
        builder: *mut ExchangeOfferWrapper,
    ) -> Result<&'a mut ExchangeOfferWrapper, Error> {
        if builder.is_null() {
            return Err(Error::new(ErrorKind::Text(
                "Offer isn't initialized".to_string(),
            )));
        }
        Ok(unsafe { &mut *builder })
    }

    pub fn add_sender_asset(&mut self, asset: AssetBundle) {
        self.sender_assets.push(asset);
    }

    pub fn add_recipient_asset(&mut self, asset: AssetBundle) {
        self.recipient_assets.push(asset);
    }

    pub fn unwrap(&self) -> ExchangeOffer {
        ExchangeOffer::new(
            &self.sender,
            self.sender_assets.clone(),
            self.sender_value,
            &self.recipient,
            self.recipient_assets.clone(),
            self.fee_strategy,
        )
    }
}

message! {
    /// `exchange` transaction.
    struct Exchange {
        const TYPE = SERVICE_ID;
        const ID = EXCHANGE_ID;

        offer:             ExchangeOffer,
        seed:              u64,
        sender_signature:  &Signature,
        data_info:         &str,
    }
}

#[derive(Clone, Debug)]
pub struct ExchangeWrapper {
    offer: ExchangeOffer,
    seed: u64,
    signature: Signature,
    data_info: String,
}

impl ExchangeWrapper {
    pub fn new(offer: ExchangeOffer, seed: u64, signature: &Signature, data_info: &str) -> Self {
        ExchangeWrapper {
            offer: offer,
            seed: seed,
            signature: *signature,
            data_info: data_info.to_string(),
        }
    }

    pub fn from_ptr<'a>(wrapper: *mut ExchangeWrapper) -> Result<&'a mut ExchangeWrapper, Error> {
        if wrapper.is_null() {
            return Err(Error::new(ErrorKind::Text(
                "transactionx isn't initialized".to_string(),
            )));
        }
        Ok(unsafe { &mut *wrapper })
    }

    pub fn unwrap(&self) -> Exchange {
        Exchange::new(
            self.offer.clone(),
            self.seed,
            &self.signature,
            self.data_info.as_str(),
            &SecretKey::zero(),
        )
    }
}