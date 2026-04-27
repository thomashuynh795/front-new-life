# API crate

This crate exposes the Axum REST API for the NFC anti-counterfeit service.

## Audit summary

### Existing routes

- `GET /health`: implemented, returns `200 OK`.
- `POST /provision`: implemented with DB-backed provisioning. Protected by `X-Admin-Key`.
- `POST /verify`: implemented with DB-backed legacy dynamic-tag verification.
- `POST /admin/tags/{tag_id}/revoke`: implemented with DB-backed admin logic. Protected by `X-Admin-Key`.
- `POST /admin/tags/{tag_id}/rotate-key`: implemented with DB-backed admin logic. Protected by `X-Admin-Key`.
- `POST /v1/products/{pid}/scan-tokens`: implemented for static NDEF one-time token generation. Protected by `X-Admin-Key`.
- `GET /v1/scan?pid=<pid>&t=<token>`: implemented for token verification and one-time consumption.

### Previous gaps found during the audit

- No one-time token flow for static NDEF tags.
- No admin protection on sensitive routes.
- No exact HTTP mapping for replay / expired / revoked / invalid token outcomes.
- No dedicated DB entity for one-time scan tokens.
- No concurrency-focused tests for one-time token consumption.
- No operator documentation for the "3 tags / 3 scans" anti-replay scenario.

## Configuration

Required environment variables:

- `ADDRESS`
- `DATABASE_URL`
- `API_DOMAIN`
- `ADMIN_API_KEY`
- `HMAC_SECRET` or `SCAN_TOKEN_SECRET`
- `MASTER_KEY_HEX` for the legacy CMAC flow

## Static NDEF anti-replay flow

### Generate future one-time scan URLs

```bash
curl -X POST "http://localhost:8101/v1/products/SKU-123/scan-tokens" \
  -H "Content-Type: application/json" \
  -H "X-Admin-Key: $ADMIN_API_KEY" \
  -d '{"count":3,"ttl_seconds":86400}'
```

Response:

- Returns `201 Created`
- Generates three distinct URLs for the same product
- Each URL contains a different one-time token

### Verify and consume a token

```bash
curl "http://localhost:8101/v1/scan?pid=SKU-123&t=<token>"
```

Status codes:

- `200 OK`: authentic, first use
- `400 Bad Request`: invalid token or invalid MAC
- `403 Forbidden`: revoked token
- `404 Not Found`: unknown token
- `409 Conflict`: replay, token already used
- `410 Gone`: expired token

## Manual test procedure with 3 NFC tags

1. Call `POST /v1/products/SKU-123/scan-tokens` with `count=3`.
2. Write each returned URL to a different NFC tag.
3. Scan tag 1:
   - first scan must return `200 OK`
   - second scan must return `409 Conflict`
4. Scan tag 2:
   - first scan must return `200 OK`
   - second scan must return `409 Conflict`
5. Scan tag 3:
   - first scan must return `200 OK`
   - second scan must return `409 Conflict`

## Notes

- The API stores `token_hash` in the database, not the clear token value.
- The token secret is never logged.
- One-time consumption relies on an atomic `UPDATE ... WHERE status = 'UNUSED'`.

## Development

```bash
cargo watch -c -x fmt -x test -x run
```
```bash
cargo watch -c -x fmt -x "clippy --all-targets --all-features -- -D warnings" -x test -x run
```