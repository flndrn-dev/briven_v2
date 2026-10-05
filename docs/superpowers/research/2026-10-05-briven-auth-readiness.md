# Briven Auth readiness for Handlr

**Checked:** 5 October 2026. **Status:** source audit; no new Auth implementation or real-phone acceptance test.

Briven has partial passkey and authenticator-code support. It does not yet meet the Handlr handoff. The work must first tighten app isolation, then complete setup, emergency codes, and account recovery. This record describes observed code and unresolved decisions, not an approved implementation design.

## Which login system is actually used

| Surface | Current implementation | Source in briven-website |
| --- | --- | --- |
| Briven dashboard | Better Auth | `apps/api/src/lib/auth.ts`; mounted by `apps/api/src/index.ts` |
| Customer apps | Custom Briven Auth routes and services | `apps/api/src/routes/auth-core-fdi.ts`; mounted by `apps/api/src/index.ts` |
| SuperTokens | Documentation reference; no Core process in the active customer login path | FDI route header and `docs/knowledge-base.md` |
| Older alternative Auth routes | Not mounted as the customer login implementation | `apps/api/src/routes/auth-v2.ts`; retired paths handled by `auth-product-retired.ts` |

An installed library or an old route file is not evidence that it handles live logins. This audit follows the actual router mounts.

## Storage during the rebuild

Live website `main` at `7098e1b` uses PostgreSQL for dashboard/control data, while customer Auth's dedicated `briven_engine` database still uses Doltgres. Its pool explicitly rejects a host named `postgres`.

Website rebuild branch `sprint3-serverless-postgres` at `62475e7` has a PostgreSQL-native Auth pool in `apps/api/src/services/auth-core/db.ts`. It is not the live website branch. The owner's current direction is serverless PostgreSQL with pgvector and AI. The July knowledge base's Doltgres requirements describe the older architecture; they cannot establish that the rebuild is already deployed.

## What exists and what remains

| Capability | Observed state | Work required before Handlr acceptance |
| --- | --- | --- |
| Authenticator setup | Backend generates a secret and an otpauth URI; a first code enables the device | QR image, copyable manual key, Handlr label, resumable settings flow and SDK methods |
| Password plus authenticator code | Password sign-in can return an MFA challenge | Bind challenge, user and session to the requesting project; limit retries and reject reused codes |
| Passkeys | SimpleWebAuthn registration and login routes; basic hosted registration screen | Stable app domain configuration, strict origin checks, discoverable credentials and verified device unlock |
| Emergency codes | No implementation found in the active schema or SDK | Show once, saved confirmation, hashed storage and atomic single use |
| Lost phone without emergency codes | No complete recovery flow found | Define support permissions and identity checks, then build and test recovery |
| Fresh verification for money actions | No complete Handlr integration contract established | Owner decision and a server-verifiable recent-authentication contract |
| Real phone and cross-device acceptance | Not demonstrated in this audit | Every device check in the handoff's definition of done |

## Security findings to carry into the design

These are static-code findings, not claims of a demonstrated production exploit.

- `auth-core-fdi.ts:sessionUserId` returns only the user ID. Setup and credential-management routes do not compare the session's tenant with the requesting project's tenant. The session helper exposes the tenant in its payload, but these routes discard it.
- TOTP service queries generally filter by user ID without a tenant predicate. The login route accepts the signed challenge's tenant without checking equality with the request's tenant.
- `mfa-challenge.ts` accepts a signed challenge when Redis is unavailable, so its single-use guarantee is lost. The route consumes a challenge before checking the code, making an ordinary mistyped code require another password sign-in.
- TOTP secrets are stored as plaintext. No persisted accepted time step or retry limit was found in the TOTP service. The design must encrypt secrets and address replay and guessing.
- `webauthn.ts:resolveWebAuthnRp` can fall back to a candidate origin outside the project's allowed origins and derives the RP ID from request values instead of immutable project configuration.
- WebAuthn options prefer, rather than require, discoverable credentials and user verification. Verification explicitly uses `requireUserVerification: false`; that does not establish the proposed Face ID/fingerprint-only assurance.
- Registration base64url-encodes `info.credential.id` again. Installed SimpleWebAuthn types already define that ID as a base64url string; preserving and matching browser IDs needs a regression check.
- WebAuthn challenge verification, deletion and credential-counter updates are separate queries. Concurrent successful requests need an atomic single-use proof. Finish routes must also reject a challenge from a different requesting project.

## Decisions still awaiting the owner

1. Must everyone set up extra login protection, or can people choose? The handoff recommends testing with willing Handlr users before any wider requirement.
2. Must users prove it is them again before withdrawing money or changing bank details?
3. Can Face ID/fingerprint take users straight in, with the code app as backup, or must they also enter a code?
4. Is recovery through emergency codes and a defined recovery process sufficient, or should text messages also be offered?

No unanswered choice is treated as approval to enable a policy. Other customer projects must keep their existing requirements during the Handlr pilot.

## Domain and recovery boundaries

The handoff confirms `handlr.sh`. The proposed passkey RP ID is `handlr.sh`, with explicitly approved first-party origins. It must be settled before enrolling anyone. A normal Briven-hosted page cannot simply create a Handlr-domain passkey; the SDK or a verified Handlr-domain Auth surface must perform the browser ceremony. Handlr app screens and Neon migration remain out of scope.

The recovery design must state what support can reset and what proof is required. Email access alone must not silently become a shortcut around the protection selected for a money account. No recovery design has been approved or tested here.

## References read

The project's `AGENTS.md`, `docs/knowledge-base.md`, its Doltgres section, and all SuperTokens URLs listed in the handoff were read before Auth work. The unavailable local Dolt reference directory was not treated as evidence.

SuperTokens' authenticator setup shows a QR and a manual secret, then verifies a code before enabling the device: [TOTP setup](https://supertokens.com/docs/additional-verification/mfa/totp/totp-for-all-users).

SuperTokens describes a passkey as a discoverable credential that may be synced or device-bound: [passkey concepts](https://supertokens.com/docs/authentication/passkeys/important-concepts). Briven must verify the browser's cryptographic proof; a signup authenticator QR is not passkey enrollment.

The [WebAuthn specification](https://www.w3.org/TR/webauthn-3/) defines RP scoping and requires validation of the credential origin. Recovery-code and fresh-verification behavior were also checked against [recovery codes](https://supertokens.com/docs/additional-verification/mfa/backup-codes) and [step-up authentication](https://supertokens.com/docs/additional-verification/mfa/step-up-auth).
