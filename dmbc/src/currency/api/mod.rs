// TODO: currency service API documentation.
#![allow(missing_docs)]

pub mod asset;
pub mod assets_intern;
pub mod blocks;
pub mod db_stats;
pub mod error;
pub mod fees;
pub mod hex;
pub mod metrics;
pub mod transaction;
pub mod wallet;
pub mod offers;
pub mod history_offers;

extern crate params;

use exonum::api::Api;
use exonum::blockchain::Blockchain;
use exonum::node::ApiSender;
use hyper::header::{AccessControlAllowHeaders, AccessControlAllowMethods, Headers};
use hyper::method::Method;
use hyper::status::StatusCode;
use iron::headers::AccessControlAllowOrigin;
use iron::prelude::*;
use router::Router;
use std::cmp;
use unicase::UniCase;

use self::asset::AssetApi;
use self::assets_intern::AssetInternApi;
use self::blocks::BlocksApi;
use self::db_stats::DbStatsApi;
use self::fees::FeesApi;
use self::hex::HexApi;
use self::metrics::MetricsApi;
use self::params::{FromValue, Params};
use self::transaction::TransactionApi;
use self::wallet::WalletApi;
use self::offers::OfferApi;
use self::history_offers::HistoryOffersApi;

const PARAMETER_OFFSET_KEY: &str = "offset";
const PARAMETER_LIMIT_KEY: &str = "limit";

#[derive(Clone)]
pub struct ServiceApi {
    pub channel: ApiSender,
    pub blockchain: Blockchain,
}

impl ServiceApi {
    /// returns a slice `&[T]` if `request` has pagination parameters.
    /// `offset` and `limit`, otherwise returns existing slice
    ///
    /// # URL request
    ///
    /// ` https://blockchain.com/api/services/cryptocurrency/v1/wallets?offset=4&limit=10 `
    pub fn apply_pagination<'a, T>(req: &mut Request, elements: &'a [T]) -> &'a [T] {
        let total_count = elements.len();
        // read url parameters
        let parameters = req.get_ref::<Params>().unwrap();
        let offset_parameter = parameters.get(PARAMETER_OFFSET_KEY);
        let limit_parameter = parameters.get(PARAMETER_LIMIT_KEY);

        // pagination parameters `offset` and `limit` should be considered together
        if offset_parameter.is_some() && limit_parameter.is_some() {
            let offset = FromValue::from_value(offset_parameter.unwrap()).unwrap_or(0);
            let limit = FromValue::from_value(limit_parameter.unwrap()).unwrap_or(total_count);

            // validate parameters for pagination
            let from = cmp::min(offset, total_count);
            let to = cmp::min(from + limit, total_count);
            return &elements[from..to];
        }

        elements
    }

    pub fn pagination_params(req: &mut Request) -> (u64, u64) {
        let parameters = req.get_ref::<Params>().unwrap();
        let offset_parameter = parameters.get(PARAMETER_OFFSET_KEY);
        let limit_parameter = parameters.get(PARAMETER_LIMIT_KEY);

        // pagination parameters `offset` and `limit` should be considered together
        if offset_parameter.is_some() && limit_parameter.is_some() {
            let offset = FromValue::from_value(offset_parameter.unwrap()).unwrap_or(0);
            let limit = FromValue::from_value(limit_parameter.unwrap()).unwrap_or(1000);
            (offset, limit)
        } else {
            (0, 1000)
        }
    }

    pub fn read_parameter<T>(req: &mut Request, parameter_key: &str, default_value: T) -> T
    where
        T: FromValue,
    {
        let parameters = req.get_ref::<Params>().unwrap();
        if let Some(parameter) = parameters.get(parameter_key) {
            return FromValue::from_value(parameter).unwrap_or(default_value);
        }

        default_value
    }

    pub fn add_option_headers(headers: &mut Headers) {
        headers.set(AccessControlAllowOrigin::Any);
        headers.set(AccessControlAllowHeaders(vec![UniCase(
            "content-type".to_owned(),
        )]));
        headers.set(AccessControlAllowMethods(vec![
            Method::Get,
            Method::Post,
            Method::Options,
        ]));
    }
}

impl Api for ServiceApi {
    fn wire(&self, router: &mut Router) {
        let self_ = self.clone();
        let api = TransactionApi {
            channel: self_.channel,
            blockchain: self_.blockchain,
        };
        api.wire(router);

        let api = AssetApi {
            blockchain: self.clone().blockchain,
        };
        api.wire(router);

        let api = WalletApi {
            blockchain: self.clone().blockchain,
        };
        api.wire(router);

        let api = HexApi {};
        api.wire(router);

        let api = AssetInternApi {};
        api.wire(router);

        let api = MetricsApi {};
        api.wire(router);

        let api = DbStatsApi {};
        api.wire(router);

        let api = FeesApi {
            blockchain: self.clone().blockchain,
        };
        api.wire(router);

        let api = BlocksApi {
            blockchain: self.clone().blockchain,
        };
        api.wire(router);

        let api = OfferApi {
            blockchain: self.clone().blockchain,
        };
        api.wire(router);

        let api = HistoryOffersApi {
            blockchain: self.clone().blockchain,
        };
        api.wire(router);

        let send_option = move |_request: &mut Request| -> IronResult<Response> {
            let mut resp = Response::with(StatusCode::Ok);
            ServiceApi::add_option_headers(&mut resp.headers);
            Ok(resp)
        };

        router.options("/*", send_option, "send_options");
    }
}
