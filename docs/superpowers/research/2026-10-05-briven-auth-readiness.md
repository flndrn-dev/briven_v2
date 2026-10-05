# Briven Auth platform readiness

**Checked:** 5 October 2026. **Status:** source audit; no new Auth implementation or real-phone acceptance test.

Briven has partial passkey and authenticator-code support. The platform still needs stronger project isolation, complete setup, emergency codes, and account recovery. The supplied application handoff describes capabilities for Briven Auth; it does not authorize building application-specific business rules. This record describes observed code and unresolved design details, not an approved implementation design.

## Platform scope and Neon foundation

The owner clarified that this work must serve Briven as a general database platform and remain as close as practical to the Neon GitHub foundation. The verified engine upstream is [neondatabase/neon](https://github.com/neondatabase/neon). Its documented architecture separates PostgreSQL compute from the storage engine; Briven's existing serverless rebuild follows that foundation.

Auth features belong in Briven's shared project configuration, API and SDK. App names, domains and policies must come from each project's settings. Handlr and mavi pay are examples of applications using the platform, not special cases in its implementation. No customer application screens, payment logic, fixed customer domain, or dedicated customer Auth fork belongs in this work.

This scope correction does not itself approve replacing the current Auth backend. The active implementations are recorded below; changes to that architecture require an explicit platform design rather than assuming the Neon engine repository supplies a complete hosted Auth service.

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

| Capability | Observed state | Work required before platform acceptance |
| --- | --- | --- |
| Authenticator setup | Backend generates a secret and an otpauth URI; a first code enables the device | QR image, copyable manual key, project-specific app label, resumable settings flow and SDK methods |
| Password plus authenticator code | Password sign-in can return an MFA challenge | Bind challenge, user and session to the requesting project; limit retries and reject reused codes |
| Passkeys | SimpleWebAuthn registration and login routes; basic hosted registration screen | Stable app domain configuration, strict origin checks, discoverable credentials and verified device unlock |
| Emergency codes | No implementation found in the active schema or SDK | Show once, saved confirmation, hashed storage and atomic single use |
| Lost phone without emergency codes | No complete recovery flow found | Define support permissions and identity checks, then build and test recovery |
| Fresh verification for sensitive actions | Owner confirmed the need using money actions as an example; reusable integration not built | A server-verifiable recent-authentication contract that any app can request |
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

## Confirmed login policy

The owner confirmed on 5 October that extra login protection is optional. Briven should recommend it, while allowing people to keep regular authentication. This replaces the earlier proposed progression toward mandatory enrollment; no automatic move to mandatory enrollment is approved.

Users who keep regular authentication should receive recurring email reminders explaining the benefit of two-step verification and linking to setup. The implementation design must define the reminder schedule, avoid duplicate emails, and stop these reminders once the user completes the recommended protection. No reminder email has been sent and no delivery schedule has been activated.

The owner confirmed the need for fresh identity verification during a session. The later scope correction makes this a general Briven Auth capability: an application can request fresh verification for its sensitive actions and check the proof on its backend. Withdrawals and bank-detail changes were examples, not platform business rules. Briven must not contain payment-specific action handling or automatically impose those rules on every project. Accepted verification methods and freshness rules belong in the platform design and project settings, consistent with optional enrollment.

## Platform design details still to resolve

The design must define how projects configure passkeys as a primary login method or as additional verification, and how their recovery policy works. These are shared platform settings, not questions about building one particular customer application. SMS fallback is not approved by the supplied feature request.

No unresolved design detail is treated as approval to activate a policy. Existing projects must keep their current login requirements during staged platform testing.

## Domain and recovery boundaries

Each project's passkey RP ID and allowed first-party origins must be configured and verified before enrollment. The RP ID must remain stable after credentials are issued; no particular customer's domain should be hard-coded. A normal Briven-hosted page cannot simply create a passkey scoped to an unrelated customer domain. The SDK or a verified Auth surface on the project's domain must perform the browser ceremony. Customer app screens and migration from another provider remain separate work.

The recovery design must state what support can reset and what proof is required. Email access alone must not silently become a shortcut around a project's selected protection. No recovery design has been approved or tested here.

## References read

The project's `AGENTS.md`, `docs/knowledge-base.md`, its Doltgres section, and all SuperTokens URLs listed in the handoff were read before Auth work. The unavailable local Dolt reference directory was not treated as evidence.

SuperTokens' authenticator setup shows a QR and a manual secret, then verifies a code before enabling the device: [TOTP setup](https://supertokens.com/docs/additional-verification/mfa/totp/totp-for-all-users).

SuperTokens describes a passkey as a discoverable credential that may be synced or device-bound: [passkey concepts](https://supertokens.com/docs/authentication/passkeys/important-concepts). Briven must verify the browser's cryptographic proof; a signup authenticator QR is not passkey enrollment.

The [WebAuthn specification](https://www.w3.org/TR/webauthn-3/) defines RP scoping and requires validation of the credential origin. Recovery-code and fresh-verification behavior were also checked against [recovery codes](https://supertokens.com/docs/additional-verification/mfa/backup-codes) and [step-up authentication](https://supertokens.com/docs/additional-verification/mfa/step-up-auth).
