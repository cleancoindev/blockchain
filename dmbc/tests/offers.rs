extern crate dmbc;
extern crate exonum;
extern crate exonum_testkit;
extern crate hyper;
extern crate iron;
extern crate iron_test;
extern crate mount;
extern crate serde_json;

pub mod dmbc_testkit;

use std::collections::HashMap;

use dmbc_testkit::{DmbcTestApiBuilder, DmbcTestKitApi};

use exonum::crypto;
use exonum::messages::Message;
use hyper::status::StatusCode;

use dmbc::currency::api::offers::{OpenOffersInfo, OpenOffersResponse};
use dmbc::currency::transactions::builders::transaction;
use dmbc::currency::assets::TradeAsset;
use dmbc::currency::wallet::Wallet;
use dmbc::currency::offers::{OpenOffers, Offer};

#[test]
fn offers() {
    let testkit = DmbcTestApiBuilder::new().create();
    let api = testkit.api();

    let (status, response): (StatusCode, OpenOffersResponse) = api.get_with_status("/v1/offers");

    assert_eq!(status, StatusCode::Ok);
    assert_eq!(
        response,
        Ok(OpenOffersInfo {
            total: 0,
            count: 0,
            offers_info: HashMap::new()
        })
    );
}


#[test]
fn offers_by_asset() {
    let fixed = 10;
    let units = 2;
    let balance = 1000;
    let meta_data = "asset";

    let (user1_pk, user1_sk) = crypto::gen_keypair();
    let (asset, info) = dmbc_testkit::create_asset(
        meta_data,
        units,
        dmbc_testkit::asset_fees(fixed, "0.0".parse().unwrap()),
        &user1_pk,
    );

    let mut testkit = DmbcTestApiBuilder::new()
        .add_wallet_value(&user1_pk, Wallet::new(balance, vec![]))
        .add_asset_to_wallet(&user1_pk, (asset.clone(), info))
        .create();
    let api = testkit.api();

    let tx_bid_offer = transaction::Builder::new()
        .keypair(user1_pk, user1_sk.clone())
        .tx_offer()
        .asset(TradeAsset::from_bundle(asset.clone(), 100))
        .data_info("bid")
        .bid_build();

    let (status, _) = api.post_tx(&tx_bid_offer);
    testkit.create_block();
    assert_eq!(status, StatusCode::Created);

    let mut open_offers = OpenOffers::new(vec![], vec![]);
    open_offers.add_bid(100, Offer::new(&user1_pk, 2, &tx_bid_offer.hash()));

    let offers = api.get_offers(&asset.id());
    assert_eq!(Some(open_offers), offers);
}
