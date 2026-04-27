# Architecture Audit

## Crates

- `crates/api`: Axum REST API, use cases, crypto services, HTTP handlers.
- `crates/database-model`: SeaORM entities shared by API and migration crate.
- `crates/database-migration`: reset-style migration binary rebuilding the schema from entities.

## Current Routes

- `GET /health`
- `POST /provision`
  - Legacy admin alias of `POST /admin/tags/enroll`.
- `POST /admin/tags/enroll`
  - Admin only.
  - Body:
    ```json
    {
      "tag_uid": "04AABBCCDD",
      "product_code": "TSHIRT-001",
      "size": "M",
      "color": "BLACK",
      "mode": "dynamic_cmac"
    }
    ```
- `POST /verify`
  - Body:
    ```json
    {
      "tag_uid": "04AABBCCDD",
      "counter": 12,
      "cmac": "A1B2C3..."
    }
    ```
- `POST /admin/tags/{tag_id}/revoke`
- `POST /admin/tags/{tag_id}/rotate-key`
- `POST /admin/tags/{tag_id}/reconfigure`
- `POST /admin/tags/{tag_id}/next-messages`
- `POST /admin/scan-tokens/{token_id}/revoke`
- `POST /v1/products/{pid}/scan-tokens`
- `GET /v1/scan?pid=<pid>&t=<token>`

## DTO Summary

- Enroll request: `tag_uid`, `product_code`, `size`, `color`, `mode`
- Enroll response:
  - `tag_id`
  - `item_id`
  - `mode`
  - `payload`
- Verify request: `tag_uid`, `counter`, `cmac`
- Verify response: `verdict`, optional `product.tag_id`
- Reconfigure request:
  - `reset_counter`
  - `rotate_key`
  - `revoke_existing_batch`
  - `token_count`
  - `ttl_seconds`
- Next-messages request:
  - `count`
  - `starting_counter`

## Use Cases

- `EnrollTagUseCase`
  - Validates `tag_uid`
  - Creates `tags` and `items`
  - Creates initial one-time token batch for `one_time_tokens`
  - Creates `audit_events`
- `VerifyTagUseCase`
  - Loads tag by `tag_uid`
  - Rejects revoked tags
  - Derives `K_tag` from `TAG_SIGNING_MASTER`, `tag_uid`, `key_version`
  - Verifies CMAC
  - Atomically updates `last_counter`
  - Stores `scan_events` for all verdicts
- `RotateKeyUseCase`
  - Rejects non-dynamic tags
  - Increments `key_version`
  - Resets `last_counter`
  - Stores `audit_events`
- `ReconfigureTagUseCase`
  - `dynamic_cmac`: reset counter and/or rotate key
  - `one_time_tokens`: revoke active batch and create a new batch
  - Stores `audit_events`
- `NextMessagesUseCase`
  - Generates deterministic future `(counter, cmac)` pairs
  - Does not update `last_counter`
- `GenerateScanTokensUseCase`
  - Creates product-level token batch and one-time tokens
- `ConsumeScanTokenUseCase`
  - Verifies signed token structure
  - Loads token row by `token_id`
  - Atomically consumes `UNUSED -> USED`
  - Stores `scan_events`
- `RevokeScanTokenUseCase`
  - Sets token status to `REVOKED`

## Database Model

- `tags`
  - `id`
  - `tag_uid` unique
  - `mode`
  - `status`
  - `key_version`
  - `last_counter`
  - timestamps
- `items`
  - `id`
  - `product_code`
  - `size`
  - `color`
  - `tag_id` unique
  - timestamps
- `scan_events`
  - `id`
  - `tag_id` nullable
  - `token_id` nullable
  - `tag_uid`
  - `product_public_id` nullable
  - `received_counter` nullable
  - `verdict`
  - `metadata`
  - `ip`
  - `user_agent`
  - `created_at`
- `scan_tokens`
  - `token_id`
  - `batch_id` nullable
  - `tag_id` nullable
  - `product_public_id`
  - `status`
  - `expires_at`
  - `used_at`
  - `revoked_at`
  - `token_hash` unique
- `token_batches`
  - `id`
  - `tag_id` nullable
  - `product_public_id`
  - `status`
  - `expires_at`
  - `revoked_at`
  - `created_at`
- `audit_events`
  - `id`
  - `tag_id` nullable
  - `event_type`
  - `metadata`
  - `created_at`

## Verification Flows

### `dynamic_cmac`

1. Client sends `tag_uid`, `counter`, `cmac` to `POST /verify`.
2. API loads the tag row.
3. API derives `K_tag = HKDF-SHA256(master=TAG_SIGNING_MASTER, salt=tag_uid_bytes, info=key_version_bytes)`.
4. API computes AES-CMAC over `tag_uid_bytes || counter_be_bytes`.
5. If CMAC is valid, API executes an atomic counter update:
   - `last_counter IS NULL OR last_counter < new_counter`
6. If update affects zero rows, verdict is `REPLAY_DETECTED`.

### `one_time_tokens`

1. API generates signed token URLs with `POST /v1/products/{pid}/scan-tokens` or tag enrollment in `one_time_tokens` mode.
2. Client opens `GET /v1/scan?pid=<pid>&t=<token>`.
3. API verifies token MAC with `TOKEN_SECRET`.
4. API atomically flips `UNUSED -> USED`.
5. A second request on the same token returns `409`.

## Placeholder Replacements Completed

- Legacy `generate_keys()` placeholder returning a fixed version and `"master-key-id-placeholder"` was removed.
- Legacy `MASTER_KEY_HEX` default fallback was replaced by explicit `TAG_SIGNING_MASTER` configuration, with legacy env fallback only for compatibility.
- `/verify` no longer trusts a non-atomic read-then-write counter update.

## HTTP Mapping

- `401 Unauthorized`
  - Missing or invalid `X-Admin-Key`
  - Invalid CMAC / invalid token signature
- `403 Forbidden`
  - Revoked one-time token
- `404 Not Found`
  - Unknown tag
  - Unknown scan token
- `409 Conflict`
  - Replay on `dynamic_cmac`
  - Replay on `one_time_tokens`
- `410 Gone`
  - Revoked tag
  - Expired one-time token
