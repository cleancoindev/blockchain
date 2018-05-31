#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#include "dmbc_capi.h"
#include "cjson.h"

#ifndef DEBUG
#define DEBUG false
#endif

const char *error_msg = "Error occured '%s'\n";
const char *output = "\nTransaction length is %lu hex %s\n";
const char *output_fodler = "output\\";

void print_hex(const uint8_t *hex, size_t length) {
    for (int i = 0; i < length; ++i) {
        fprintf(stdout, "%02x", hex[i]);
    }
    puts("");
}

void write_hex_to_file(const char *fname, uint8_t *hex, size_t length) {
    FILE *f = fopen(fname, "w");
    if (NULL == f) {
        fprintf(stderr, "Error opening file %s\n", fname);
        exit(1);
    }

    for (int i = 0; i < length; ++i) {
        fprintf(f, "%02x", hex[i]);
    }

    fclose(f);
}

cJSON * read_inputs(const char *fname) {
    FILE *f = fopen(fname, "r");

    if (NULL == f) {
        fprintf(stderr, "Error opening file %s\n", fname);
        exit(1);
    }

    fseek(f, 0, SEEK_END);
    size_t string_size = ftell(f);
    rewind(f);

    char *buffer = (char *)malloc(sizeof(char) * (string_size + 1));
    size_t read_size = fread(buffer, sizeof(char), string_size, f);
    buffer[string_size] = '\0';

    if (string_size != read_size) {
        free(buffer);
        buffer = NULL;
        fclose(f);
        fprintf(stderr, "Error reading file %s\n", fname);
        exit(1);
    }

    fclose(f);

    cJSON *inputs = cJSON_Parse(buffer);
    if (NULL == inputs) {
        const char *error_ptr = cJSON_GetErrorPtr();
        if (error_ptr != NULL)
        {
            fprintf(stderr, "Error before: %s\n", error_ptr);
        }
        exit(1);
    }

    free(buffer);

    return inputs;
}

void add_assets() {
    cJSON *inputs = read_inputs("./inputs/add_assets.json");
    const cJSON *pub_key_json = cJSON_GetObjectItemCaseSensitive(inputs, "public_key");
    const cJSON *seed_json = cJSON_GetObjectItemCaseSensitive(inputs, "seed");
    const cJSON *assets = cJSON_GetObjectItemCaseSensitive(inputs, "assets");
    const cJSON *asset = NULL;

    uint64_t seed = seed_json->valueint;
    const char *public_key = pub_key_json->valuestring;

    dmbc_error *err = dmbc_error_new();
    dmbc_tx_add_assets *tx = dmbc_tx_add_assets_create(public_key, seed, err);
    if (NULL == tx) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_error;
    }

    cJSON_ArrayForEach(asset, assets) {

        const cJSON *fees_json = cJSON_GetObjectItemCaseSensitive(asset, "fees");
        const cJSON *trade_json = cJSON_GetObjectItemCaseSensitive(fees_json, "trade");
        const cJSON *exchange_json = cJSON_GetObjectItemCaseSensitive(fees_json, "exchange");
        const cJSON *transfer_json = cJSON_GetObjectItemCaseSensitive(fees_json, "transfer");

        const cJSON *trade_fixed = cJSON_GetObjectItemCaseSensitive(trade_json, "fixed");
        const cJSON *trade_fraction = cJSON_GetObjectItemCaseSensitive(trade_json, "fraction");
        const cJSON *exchange_fixed = cJSON_GetObjectItemCaseSensitive(exchange_json, "fixed");
        const cJSON *exchange_fraction = cJSON_GetObjectItemCaseSensitive(exchange_json, "fraction");
        const cJSON *transfer_fixed = cJSON_GetObjectItemCaseSensitive(transfer_json, "fixed");
        const cJSON *transfer_fraction = cJSON_GetObjectItemCaseSensitive(transfer_json, "fraction");

        dmbc_fees *fees = dmbc_fees_create(
            trade_fixed->valueint, 
            trade_fraction->valuestring,
            exchange_fixed->valueint,
            exchange_fraction->valuestring,
            transfer_fixed->valueint,
            transfer_fraction->valuestring,
            err
        );

        if (NULL == fees) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            goto free_tx;
        }

        const cJSON *data = cJSON_GetObjectItemCaseSensitive(asset, "data");
        const cJSON *amount = cJSON_GetObjectItemCaseSensitive(asset, "amount");
        const cJSON *receiver = cJSON_GetObjectItemCaseSensitive(asset, "receiver");

        if (!dmbc_tx_add_assets_add_asset(
            tx, 
            data->valuestring, 
            amount->valueint, 
            fees, 
            receiver->valuestring, 
            err)
        ) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            dmbc_fees_free(fees);
            goto free_tx;
        }

        dmbc_fees_free(fees);
    }
    
    size_t length = 0;
    uint8_t *buffer = dmbc_tx_add_assets_into_bytes(tx, &length, err);
    if (NULL == buffer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_tx;
    }

    write_hex_to_file("./output/add_assets", buffer, length);

    dmbc_bytes_free(buffer, length);
free_tx:
    dmbc_tx_add_asset_free(tx);
free_error:
    dmbc_error_free(err);

    cJSON_Delete(inputs);
}

void delete_assets() {
    cJSON *inputs = read_inputs("./inputs/delete_assets.json");
    const cJSON *pub_key_json = cJSON_GetObjectItemCaseSensitive(inputs, "public_key");
    const cJSON *seed_json = cJSON_GetObjectItemCaseSensitive(inputs, "seed");
    const cJSON *assets = cJSON_GetObjectItemCaseSensitive(inputs, "assets");
    const cJSON *asset = NULL;

    uint64_t seed = seed_json->valueint;
    const char *public_key = pub_key_json->valuestring;

    dmbc_error *err = dmbc_error_new();
    dmbc_tx_delete_assets *tx = dmbc_tx_delete_assets_create(public_key, seed, err);
    if (NULL == tx) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_error;
    }

    cJSON_ArrayForEach(asset, assets) {
        cJSON *id = cJSON_GetObjectItemCaseSensitive(asset, "id");
        cJSON *amount = cJSON_GetObjectItemCaseSensitive(asset, "amount");

        dmbc_asset *asset = dmbc_asset_create(id->valuestring, amount->valueint, err);
        if (NULL == asset) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            goto free_tx;
        }

        if (!dmbc_tx_delete_assets_add_asset(tx, asset, err)) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            dmbc_asset_free(asset);
            goto free_tx;
        }

        dmbc_asset_free(asset);
    }
    
    size_t length = 0;
    uint8_t *buffer = dmbc_tx_delete_assets_into_bytes(tx, &length, err);
    if (NULL == buffer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_tx;
    }

    write_hex_to_file("./output/delete_assets", buffer, length);

    dmbc_bytes_free(buffer, length);
free_tx:
    dmbc_tx_delete_assets_free(tx);
free_error:
    dmbc_error_free(err);

    cJSON_Delete(inputs);
}

void transfer() {
    cJSON *inputs = read_inputs("./inputs/transfer.json");
    const cJSON *from_key_json = cJSON_GetObjectItemCaseSensitive(inputs, "from");
    const cJSON *to_key_json = cJSON_GetObjectItemCaseSensitive(inputs, "to");
    const cJSON *seed_json = cJSON_GetObjectItemCaseSensitive(inputs, "seed");
    const cJSON *assets = cJSON_GetObjectItemCaseSensitive(inputs, "assets");
    const cJSON *memo = cJSON_GetObjectItemCaseSensitive(inputs, "memo");
    const cJSON *amount_json = cJSON_GetObjectItemCaseSensitive(inputs, "amount");
    const cJSON *asset = NULL;

    uint64_t seed = seed_json->valueint;
    uint64_t amount = amount_json->valueint;
    const char *from_public_key = from_key_json->valuestring;
    const char *to_public_key = to_key_json->valuestring;

    dmbc_error *err = dmbc_error_new();
    dmbc_tx_transfer *tx = dmbc_tx_transfer_create(from_public_key, to_public_key, amount, seed, memo->valuestring, err);
    if (NULL == tx) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_error;
    }

    cJSON_ArrayForEach(asset, assets) {
        cJSON *id = cJSON_GetObjectItemCaseSensitive(asset, "id");
        cJSON *amount = cJSON_GetObjectItemCaseSensitive(asset, "amount");

        dmbc_asset *asset = dmbc_asset_create(id->valuestring, amount->valueint, err);
        if (NULL == asset) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            goto free_tx;
        }

        if (!dmbc_tx_transfer_add_asset(tx, asset, err)) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            dmbc_asset_free(asset);
            goto free_tx;
        }

        dmbc_asset_free(asset);
    }
    
    size_t length = 0;
    uint8_t *buffer = dmbc_tx_transfer_into_bytes(tx, &length, err);
    if (NULL == buffer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_tx;
    }

    write_hex_to_file("./output/transfer", buffer, length);

    dmbc_bytes_free(buffer, length);
free_tx:
    dmbc_tx_transfer_free(tx);
free_error:
    dmbc_error_free(err);

    cJSON_Delete(inputs);
}

void exchange() {
    cJSON *inputs = read_inputs("./inputs/exchange.json");
    const cJSON *offer_json = cJSON_GetObjectItemCaseSensitive(inputs, "offer");
    const cJSON *sender_key_json = cJSON_GetObjectItemCaseSensitive(offer_json, "sender");
    const cJSON *recipient_key_json = cJSON_GetObjectItemCaseSensitive(offer_json, "recipient");
    const cJSON *recipient_assets = cJSON_GetObjectItemCaseSensitive(offer_json, "recipient_assets");
    const cJSON *sender_assets = cJSON_GetObjectItemCaseSensitive(offer_json, "sender_assets");
    const cJSON *sender_value_json = cJSON_GetObjectItemCaseSensitive(offer_json, "sender_value");
    const cJSON *fee_strategy_json = cJSON_GetObjectItemCaseSensitive(offer_json, "fee_strategy");

    const cJSON *memo = cJSON_GetObjectItemCaseSensitive(inputs, "memo");
    const cJSON *seed_json = cJSON_GetObjectItemCaseSensitive(inputs, "seed");
    const cJSON *signature = cJSON_GetObjectItemCaseSensitive(inputs, "sender_signature");

    const cJSON *asset = NULL;

    const char *sender_public_key = sender_key_json->valuestring;
    const char *recipient_public_key = recipient_key_json->valuestring;
    uint64_t sender_value = sender_value_json->valueint;
    u_int8_t fee_strategy = fee_strategy_json->valueint;

    dmbc_error *err = dmbc_error_new();
    dmbc_exchange_offer *offer = dmbc_exchange_offer_create(sender_public_key, sender_value, recipient_public_key, fee_strategy, err);
    if (NULL == offer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_error;
    }

    cJSON_ArrayForEach(asset, recipient_assets) {
        cJSON *id = cJSON_GetObjectItemCaseSensitive(asset, "id");
        cJSON *amount = cJSON_GetObjectItemCaseSensitive(asset, "amount");

        dmbc_asset *asset = dmbc_asset_create(id->valuestring, amount->valueint, err);
        if (NULL == asset) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            goto free_offer;
        }

        if (!dmbc_exchange_offer_recipient_add_asset(offer, asset, err)) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            dmbc_asset_free(asset);
            goto free_offer;
        }

        dmbc_asset_free(asset);
    }

    cJSON_ArrayForEach(asset, sender_assets) {
        cJSON *id = cJSON_GetObjectItemCaseSensitive(asset, "id");
        cJSON *amount = cJSON_GetObjectItemCaseSensitive(asset, "amount");

        dmbc_asset *asset = dmbc_asset_create(id->valuestring, amount->valueint, err);
        if (NULL == asset) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            goto free_offer;
        }

        if (!dmbc_exchange_offer_sender_add_asset(offer, asset, err)) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            dmbc_asset_free(asset);
            goto free_offer;
        }

        dmbc_asset_free(asset);
    }

    dmbc_tx_exchange *tx = dmbc_tx_exchange_create(offer, signature->valuestring, seed_json->valueint, memo->valuestring, err);
    if (NULL == tx) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_offer;
    }

    size_t length = 0;
    uint8_t *buffer = dmbc_tx_exchange_into_bytes(tx, &length, err);
    if (NULL == buffer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_tx;
    }

    write_hex_to_file("./output/exchange", buffer, length);

    dmbc_bytes_free(buffer, length);
free_tx:
    dmbc_tx_exchange_free(tx);
free_offer:
    dmbc_exchange_offer_free(offer);
free_error:
    dmbc_error_free(err);

    cJSON_Delete(inputs);
}

void exchange_intermediary() {
    cJSON *inputs = read_inputs("./inputs/exchange_intermediary.json");
    const cJSON *offer_json = cJSON_GetObjectItemCaseSensitive(inputs, "offer");

    const cJSON *intemediary_json = cJSON_GetObjectItemCaseSensitive(offer_json, "intermediary");
    const cJSON *interm_wallet = cJSON_GetObjectItemCaseSensitive(intemediary_json, "wallet");
    const cJSON *interm_commission = cJSON_GetObjectItemCaseSensitive(intemediary_json, "commission");

    const cJSON *sender_key_json = cJSON_GetObjectItemCaseSensitive(offer_json, "sender");
    const cJSON *recipient_key_json = cJSON_GetObjectItemCaseSensitive(offer_json, "recipient");
    const cJSON *recipient_assets = cJSON_GetObjectItemCaseSensitive(offer_json, "recipient_assets");
    const cJSON *sender_assets = cJSON_GetObjectItemCaseSensitive(offer_json, "sender_assets");
    const cJSON *sender_value_json = cJSON_GetObjectItemCaseSensitive(offer_json, "sender_value");
    const cJSON *fee_strategy_json = cJSON_GetObjectItemCaseSensitive(offer_json, "fee_strategy");

    const cJSON *memo = cJSON_GetObjectItemCaseSensitive(inputs, "memo");
    const cJSON *seed_json = cJSON_GetObjectItemCaseSensitive(inputs, "seed");
    const cJSON *sender_signature_json = cJSON_GetObjectItemCaseSensitive(inputs, "sender_signature");
    const cJSON *intermediary_signature_json = cJSON_GetObjectItemCaseSensitive(inputs, "intermediary_signature");

    const cJSON *asset = NULL;

    const char *sender_public_key = sender_key_json->valuestring;
    const char *recipient_public_key = recipient_key_json->valuestring;
    const char *intermediary_key = interm_wallet->valuestring;

    dmbc_error *err = dmbc_error_new();
    dmbc_intermediary *intermediary = dmbc_intermediary_create(intermediary_key, interm_commission->valueint);
    if (NULL == intermediary) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_error;
    }

    dmbc_exchange_offer_intermediary *offer = dmbc_exchange_offer_intermediary_create(
        intermediary, 
        sender_public_key, 
        sender_value_json->valueint, 
        recipient_public_key, 
        fee_strategy_json->valueint,
        err
    );
    if (NULL == offer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_intermediary;
    }

    cJSON_ArrayForEach(asset, recipient_assets) {
        cJSON *id = cJSON_GetObjectItemCaseSensitive(asset, "id");
        cJSON *amount = cJSON_GetObjectItemCaseSensitive(asset, "amount");

        dmbc_asset *asset = dmbc_asset_create(id->valuestring, amount->valueint, err);
        if (NULL == asset) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            goto free_offer;
        }

        if (!dmbc_exchange_offer_intermediary_recipient_add_asset(offer, asset, err)) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            dmbc_asset_free(asset);
            goto free_offer;
        }

        dmbc_asset_free(asset);
    }

    cJSON_ArrayForEach(asset, sender_assets) {
        cJSON *id = cJSON_GetObjectItemCaseSensitive(asset, "id");
        cJSON *amount = cJSON_GetObjectItemCaseSensitive(asset, "amount");

        dmbc_asset *asset = dmbc_asset_create(id->valuestring, amount->valueint, err);
        if (NULL == asset) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            goto free_offer;
        }

        if (!dmbc_exchange_offer_intermediary_sender_add_asset(offer, asset, err)) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            dmbc_asset_free(asset);
            goto free_offer;
        }

        dmbc_asset_free(asset);
    }

    const char *signature = sender_signature_json->valuestring;
    const char *intermediary_signature = intermediary_signature_json->valuestring;

    dmbc_tx_exchange_intermediary *tx = dmbc_tx_exchange_intermediary_create(
        offer, 
        signature, 
        intermediary_signature, 
        seed_json->valueint, 
        memo->valuestring, 
        err
    );
    if (NULL == tx) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_offer;
    }

    size_t length = 0;
    uint8_t *buffer = dmbc_tx_exchange_intermediary_into_bytes(tx, &length, err);
    if (NULL == buffer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_tx;
    }

    write_hex_to_file("./output/exchange_intermediary", buffer, length);

    dmbc_bytes_free(buffer, length);
free_tx:
    dmbc_tx_exchange_intermediary_free(tx);
free_offer:
    dmbc_exchange_offer_intermediary_free(offer);
free_intermediary:
    dmbc_intermediary_free(intermediary);
free_error:
    dmbc_error_free(err);

    cJSON_Delete(inputs);
}

void trade() {
    cJSON *inputs = read_inputs("./inputs/trade.json");
    const cJSON *offer_json = cJSON_GetObjectItemCaseSensitive(inputs, "offer");

    const cJSON *seller_key_json = cJSON_GetObjectItemCaseSensitive(offer_json, "seller");
    const cJSON *buyer_key_json = cJSON_GetObjectItemCaseSensitive(offer_json, "buyer");
    const cJSON *assets_json = cJSON_GetObjectItemCaseSensitive(offer_json, "assets");
    const cJSON *fee_strategy_json = cJSON_GetObjectItemCaseSensitive(offer_json, "fee_strategy");

    const cJSON *seed_json = cJSON_GetObjectItemCaseSensitive(inputs, "seed");
    const cJSON *seller_signature_json = cJSON_GetObjectItemCaseSensitive(inputs, "seller_signature");

    const cJSON *asset = NULL;

    const char *seller_public_key = seller_key_json->valuestring;
    const char *buyer_public_key = buyer_key_json->valuestring;

    dmbc_error *err = dmbc_error_new();
    dmbc_trade_offer *offer = dmbc_trade_offer_create(seller_public_key, buyer_public_key, fee_strategy_json->valueint, err);
    if (NULL == offer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_error;
    }

    cJSON_ArrayForEach(asset, assets_json) {
        cJSON *id = cJSON_GetObjectItemCaseSensitive(asset, "id");
        cJSON *amount = cJSON_GetObjectItemCaseSensitive(asset, "amount");
        cJSON *price = cJSON_GetObjectItemCaseSensitive(asset, "price");

        dmbc_trade_asset *asset = dmbc_trade_asset_create(id->valuestring, amount->valueint, price->valueint, err);
        if (NULL == asset) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            goto free_offer;
        }

        if (!dmbc_trade_offer_add_asset(offer, asset, err)) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            dmbc_trade_asset_free(asset);
            goto free_offer;
        }

        dmbc_trade_asset_free(asset);
    }

    const char *signature = seller_signature_json->valuestring;
    dmbc_tx_trade *tx = dmbc_tx_trade_create(offer, signature, seed_json->valueint, err);
    if (NULL == tx) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_offer;
    }

    size_t length = 0;
    uint8_t *buffer = dmbc_tx_trade_into_bytes(tx, &length, err);
    if (NULL == buffer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_tx;
    }

    write_hex_to_file("./output/trade", buffer, length);

    dmbc_bytes_free(buffer, length);

free_tx:
    dmbc_tx_trade_free(tx);
free_offer:
    dmbc_trade_offer_free(offer);
free_error:
    dmbc_error_free(err);

    cJSON_Delete(inputs);
}

void trade_intermediary() {
    cJSON *inputs = read_inputs("./inputs/trade_intermediary.json");
    const cJSON *offer_json = cJSON_GetObjectItemCaseSensitive(inputs, "offer");

    const cJSON *intermediary_json = cJSON_GetObjectItemCaseSensitive(offer_json, "intermediary");
    const cJSON *interm_wallet = cJSON_GetObjectItemCaseSensitive(intermediary_json, "wallet");
    const cJSON *interm_commission = cJSON_GetObjectItemCaseSensitive(intermediary_json, "commission");

    const cJSON *seller_key_json = cJSON_GetObjectItemCaseSensitive(offer_json, "seller");
    const cJSON *buyer_key_json = cJSON_GetObjectItemCaseSensitive(offer_json, "buyer");
    const cJSON *assets_json = cJSON_GetObjectItemCaseSensitive(offer_json, "assets");
    const cJSON *fee_strategy_json = cJSON_GetObjectItemCaseSensitive(offer_json, "fee_strategy");

    const cJSON *seed_json = cJSON_GetObjectItemCaseSensitive(inputs, "seed");
    const cJSON *seller_signature_json = cJSON_GetObjectItemCaseSensitive(inputs, "seller_signature");
    const cJSON *intermediary_signature_json = cJSON_GetObjectItemCaseSensitive(inputs, "intermediary_signature");
    const cJSON *memo = cJSON_GetObjectItemCaseSensitive(inputs, "memo");

    const cJSON *asset = NULL;

    const char *seller_public_key = seller_key_json->valuestring;
    const char *buyer_public_key = buyer_key_json->valuestring;
    const char *intermediary_key = interm_wallet->valuestring;
    const char *signature = seller_signature_json->valuestring;
    const char *intermediary_signature = intermediary_signature_json->valuestring;

    dmbc_error *err = dmbc_error_new();
    dmbc_intermediary *intermediary = dmbc_intermediary_create(intermediary_key, interm_commission->valueint);
    if (NULL == intermediary) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_error;
    }
    dmbc_trade_offer_intermediary *offer = dmbc_trade_offer_intermediary_create(
        intermediary, 
        seller_public_key, 
        buyer_public_key, 
        fee_strategy_json->valueint, 
        err
    );
    if (NULL == offer) {
        const char *msg = dmbc_error_message(err);
        if (NULL != msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_intermediary;
    }

    cJSON_ArrayForEach(asset, assets_json) {
        cJSON *id = cJSON_GetObjectItemCaseSensitive(asset, "id");
        cJSON *amount = cJSON_GetObjectItemCaseSensitive(asset, "amount");
        cJSON *price = cJSON_GetObjectItemCaseSensitive(asset, "price");

        dmbc_trade_asset *asset = dmbc_trade_asset_create(id->valuestring, amount->valueint, price->valueint, err);
        if (NULL == asset) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            goto free_offer;
        }

        if (!dmbc_trade_offer_intermediary_add_asset(offer, asset, err)) {
            const char *msg = dmbc_error_message(err);
            if (NULL != msg) {
                fprintf(stderr, error_msg, msg);
            }
            dmbc_trade_asset_free(asset);
            goto free_offer;
        }

        dmbc_trade_asset_free(asset);
    }

    dmbc_tx_trade_intermediary *tx = dmbc_tx_trade_intermediary_create(
        offer, 
        signature, 
        intermediary_signature, 
        seed_json->valueint, 
        memo->valuestring, 
        err
    );
    if (NULL == tx) {
        const char *msg = dmbc_error_message(err);
        if (NULL == msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_offer;
    }

    size_t length = 0;
    uint8_t *buffer = dmbc_tx_trade_intermediary_into_bytes(tx, &length, err);
    if (NULL == buffer) {
        const char *msg = dmbc_error_message(err);
        if (NULL == msg) {
            fprintf(stderr, error_msg, msg);
        }
        goto free_tx;
    }

    write_hex_to_file("./output/trade_intermediary", buffer, length);

    dmbc_bytes_free(buffer, length);

free_tx:
    dmbc_tx_trade_intermediary_free(tx);
free_offer:
    dmbc_trade_offer_intermediary_free(offer);
free_intermediary:
    dmbc_intermediary_free(intermediary);
free_error:
    dmbc_error_free(err);

    cJSON_Delete(inputs);
}

int main(int argc, char *argv[]) {
    const char *usage = "Please specify the transaction type: app TRANSACTION\nTRANSACTIONS:\n\n \
    add_assets\n \
    delete_assets\n \
    transfer\n \
    exchange\n \
    exchange_intermediary\n \
    trade\n \
    trade_intermediary\n";

    if (argc < 2) {
        puts(usage);
        return -1;
    }
    const char *tx_names[] = {
        "add_assets",
        "delete_assets",
        "transfer",
        "exchange",
        "exchange_intermediary",
        "trade",
        "trade_intermediary"
    };

    void (*fs[])(void) = {
        add_assets,
        delete_assets,
        transfer,
        exchange,
        exchange_intermediary,
        trade,
        trade_intermediary
    };

    for (int i = 0; i < 7; ++i) {
        if (strcmp(argv[1], tx_names[i]) == 0) {
            fs[i]();
            return 0;
        }
    }

    puts(usage);
    return -1;
}