# How To Test

## Environment

Required variables:

- `DATABASE_URL`
- `ADMIN_KEY`
- `TAG_SIGNING_MASTER`
- `TOKEN_SECRET`
- `API_DOMAIN`
- `ADDRESS`

Optional variables:

- `DEFAULT_SCAN_TOKEN_BATCH_SIZE`
- `DEFAULT_SCAN_TOKEN_TTL_SECONDS`

Compatibility fallbacks still accepted:

- `ADMIN_API_KEY`
- `MASTER_KEY_HEX`
- `SCAN_TOKEN_SECRET`
- `HMAC_SECRET`

## Rebuild Database

```bash
cargo run -p migration --bin reset_database
```

## Start API

```bash
cargo run -p api
```

## NFC Payload Format

### Mode `dynamic_cmac`

Chip-side state to store:

- `tag_uid`: chip UID, uppercase hex
- `key_version`: integer returned by enroll/reconfigure
- `counter`: starts at `0`

Message authenticated by the chip or test tool:

- binary payload used for CMAC: `tag_uid_bytes || counter_be_bytes`
- API payload:
  ```json
  {
    "tag_uid": "04AABBCCDD",
    "counter": 1,
    "cmac": "A1B2C3D4..."
  }
  ```

### Mode `one_time_tokens`

Static NDEF URL payload to write on the chip:

- `https://<api-domain>/v1/scan?pid=<product_code>&t=<signed_token>`

Each token is single-use:

- first scan: `200 OK`
- second scan: `409 Conflict`

## Enroll Dynamic Tag

```bash
curl -X POST http://localhost:8101/admin/tags/enroll \
  -H "Content-Type: application/json" \
  -H "X-Admin-Key: $ADMIN_KEY" \
  -d '{
    "tag_uid": "04AABBCCDD",
    "product_code": "TSHIRT-001",
    "size": "M",
    "color": "BLACK",
    "mode": "dynamic_cmac"
  }'
```

## Generate Dynamic Test Vectors

```bash
curl -X POST http://localhost:8101/admin/tags/<tag_id>/next-messages \
  -H "Content-Type: application/json" \
  -H "X-Admin-Key: $ADMIN_KEY" \
  -d '{"count":3}'
```

Expected behavior:

- does not change `last_counter`
- returns `counter` and `cmac`
- revoked tag returns `410 Gone`
- missing tag returns `404 Not Found`

## Verify Dynamic Messages

```bash
curl -X POST http://localhost:8101/verify \
  -H "Content-Type: application/json" \
  -d '{
    "tag_uid": "04AABBCCDD",
    "counter": 1,
    "cmac": "<cmac>"
  }'
```

Expected behavior:

- first valid message: `200 OK`
- invalid CMAC: `401 Unauthorized`
- replayed counter: `409 Conflict`
- revoked tag: `410 Gone`
- unknown tag: `404 Not Found`

## Clone / Replay Scenario

1. Enroll one dynamic tag.
2. Call `/admin/tags/{tag_id}/next-messages` with `count=1`.
3. Copy the same `(tag_uid, counter, cmac)` to two physical tags or two curl calls.
4. First `POST /verify` must return `200`.
5. Second identical `POST /verify` must return `409`.

## Rotate Dynamic Key

Policy implemented:

- immediate rejection of the old key version
- no compatibility window
- `rotate-key` also resets `last_counter`

Implication:

- rewrite the chip immediately after rotation with the new `key_version`

Example:

```bash
curl -X POST http://localhost:8101/admin/tags/<tag_id>/rotate-key \
  -H "X-Admin-Key: $ADMIN_KEY"
```

## Reconfigure Dynamic Tag

```bash
curl -X POST http://localhost:8101/admin/tags/<tag_id>/reconfigure \
  -H "Content-Type: application/json" \
  -H "X-Admin-Key: $ADMIN_KEY" \
  -d '{
    "reset_counter": true,
    "rotate_key": true
  }'
```

## Generate One-Time Tokens for a Product

```bash
curl -X POST http://localhost:8101/v1/products/TSHIRT-001/scan-tokens \
  -H "Content-Type: application/json" \
  -H "X-Admin-Key: $ADMIN_KEY" \
  -d '{
    "count": 3,
    "ttl_seconds": 86400
  }'
```

## Enroll Tag in `one_time_tokens` Mode

```bash
curl -X POST http://localhost:8101/admin/tags/enroll \
  -H "Content-Type: application/json" \
  -H "X-Admin-Key: $ADMIN_KEY" \
  -d '{
    "tag_uid": "04EEFF1122",
    "product_code": "TSHIRT-001",
    "size": "M",
    "color": "BLACK",
    "mode": "one_time_tokens"
  }'
```

The response includes an initial token batch and URLs ready to write into NDEF records.

## Scan One-Time Token

```bash
curl "http://localhost:8101/v1/scan?pid=TSHIRT-001&t=<token>"
```

Expected behavior:

- first scan: `200 OK`
- second scan: `409 Conflict`
- revoked token: `403 Forbidden`
- expired token: `410 Gone`

## Revoke One-Time Token

```bash
curl -X POST http://localhost:8101/admin/scan-tokens/<token_id>/revoke \
  -H "X-Admin-Key: $ADMIN_KEY"
```

## Reconfigure One-Time Token Tag

```bash
curl -X POST http://localhost:8101/admin/tags/<tag_id>/reconfigure \
  -H "Content-Type: application/json" \
  -H "X-Admin-Key: $ADMIN_KEY" \
  -d '{
    "revoke_existing_batch": true,
    "token_count": 5,
    "ttl_seconds": 86400
  }'
```

Expected behavior:

- active unused tokens from previous batches are revoked
- a new batch is returned

## Full Local Validation

```bash
cargo fmt --check
cargo test
```
