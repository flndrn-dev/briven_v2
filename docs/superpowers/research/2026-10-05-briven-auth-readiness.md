# Briven Auth platform readiness

**Checked:** 5 October 2026. **Status:** shared Auth implementation and automated checks on the PostgreSQL rebuild branch; not deployed or accepted on a real phone.

Briven's rebuild branch now includes stronger project isolation, verified passkey ceremonies, encrypted authenticator setup, single-use emergency codes and weekly protection reminder controls. Complete setup screens, recovery restrictions and real-device acceptance remain. The supplied application handoff describes shared Briven Auth capabilities; it does not authorize application-specific business rules.

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

Website rebuild branch `sprint3-serverless-postgres` at `942c8e3` has a PostgreSQL-native Auth pool in `apps/api/src/services/auth-core/db.ts` and the shared Auth changes described below. It is not the live website branch. The owner's current direction is serverless PostgreSQL with pgvector and AI. The July knowledge base's Doltgres requirements describe the older architecture; they cannot establish that the rebuild is already deployed.

## What exists and what remains

| Capability | Observed state | Work required before platform acceptance |
| --- | --- | --- |
| Authenticator setup | Encrypted tenant-scoped secrets, project issuer label, resumable pending setup and typed SDK methods | QR image, copyable manual key, complete account-settings flow and migration sweep for old plaintext rows |
| Password plus authenticator code | PostgreSQL-backed project-scoped challenge, limited retries, one-use codes and atomic session creation | Consistent enrolled-factor policy for email-code, magic-link, social and SSO login paths |
| Passkeys | Strict approved-origin and RP checks, discoverable credentials, required user verification and atomic proof/session creation | Immutable project RP configuration and real-device/cross-device acceptance |
| Emergency codes | Random codes returned once, scoped hashes stored, atomic single use and short recovery-marked session | Saved confirmation and recovery-only route restrictions with re-enrollment flow |
| Lost phone without emergency codes | No complete recovery flow | Define support permissions and identity checks, then build and test recovery |
| Weekly protection reminders | Project-branded weekly sender, dashboard controls, user preference endpoint and SDK; default off | Production SMTP verification, a working project security page and customer preference UI before activation |
| Fresh verification for sensitive actions | Owner confirmed the need using money actions as an example; reusable integration not built | A server-verifiable recent-authentication contract that any app can request |
| Real phone and cross-device acceptance | Not demonstrated in this audit | Every device check in the handoff's definition of done |

## Findings from the original source audit

These static findings describe the original rebuild baseline at `62475e7`, not a demonstrated production exploit. The fixes below are on the rebuild branch and have not been deployed to the live customer Auth service.

- `auth-core-fdi.ts:sessionUserId` returns only the user ID. Setup and credential-management routes do not compare the session's tenant with the requesting project's tenant. The session helper exposes the tenant in its payload, but these routes discard it.
- TOTP service queries generally filter by user ID without a tenant predicate. The login route accepts the signed challenge's tenant without checking equality with the request's tenant.
- `mfa-challenge.ts` accepts a signed challenge when Redis is unavailable, so its single-use guarantee is lost. The route consumes a challenge before checking the code, making an ordinary mistyped code require another password sign-in.
- TOTP secrets are stored as plaintext. No persisted accepted time step or retry limit was found in the TOTP service. The design must encrypt secrets and address replay and guessing.
- `webauthn.ts:resolveWebAuthnRp` can fall back to a candidate origin outside the project's allowed origins and derives the RP ID from request values instead of immutable project configuration.
- WebAuthn options prefer, rather than require, discoverable credentials and user verification. Verification explicitly uses `requireUserVerification: false`; that does not establish the proposed Face ID/fingerprint-only assurance.
- Registration base64url-encodes `info.credential.id` again. Installed SimpleWebAuthn types already define that ID as a base64url string; preserving and matching browser IDs needs a regression check.
- WebAuthn challenge verification, deletion and credential-counter updates are separate queries. Concurrent successful requests need an atomic single-use proof. Finish routes must also reject a challenge from a different requesting project.

The foundation changes address project/session comparisons, tenant predicates, persisted and retry-limited challenges, strict approved-origin validation, required discoverable credentials and user verification, canonical credential IDs and transactional consumption. Authenticator changes encrypt new secrets, upgrade old secrets on successful use or resumed pending setup, reject replayed time steps and bound failed factor attempts across new tickets. A full legacy-secret migration sweep and immutable RP settings remain release work.

## Implementation evidence and release gates

The shared website changes are in commits `14703b1` (project ownership), `0363b54` (MFA challenges), `2c4d9c5` (verified passkeys), `67c7303` (PostgreSQL/API contracts), `fb43b83` (authenticators and emergency codes), and `942c8e3` (weekly reminders). API, Auth SDK and web typechecks pass with no errors. Focused lint and API/web production builds pass.

The helper and SDK suite passes 113 tests. Database-dependent tests were also run separately against a disposable local PostgreSQL database: 33 Auth verification tests, seven weekly reminder tests and two function-metrics tests pass. Reminder tests use an injected sender and synthetic recipients; no real email was sent. Passkey tests verify signed synthetic authenticator responses; they do not establish Face ID, fingerprint or cross-device behavior on a real phone.

The actual reminder form was checked in Brave through a local React preview with production CSS and a fake settings API. The weekly toggle and successful save feedback work. This is component evidence, not proof of a deployed project-admin path.

Before deployment, finish QR/manual-key setup and saved-code confirmation, restrict recovery-marked sessions to recovery routes, define and test recovery without backup codes and support permissions, apply factor policy consistently across primary login methods, establish stable RP configuration, sweep legacy secrets, and demonstrate the handoff's real-device checks. The generic fresh-verification contract also remains unbuilt. No Auth production cutover is claimed.

## Confirmed login policy

The owner confirmed on 5 October that extra login protection is optional. Briven should recommend it, while allowing people to keep regular authentication. This replaces the earlier proposed progression toward mandatory enrollment; no automatic move to mandatory enrollment is approved.

The owner selected **once a week**. Users who keep regular authentication can receive a project-branded reminder linking to account security. The implementation waits seven days after the later of signup or project activation, reserves each attempt for seven days to avoid duplicate sends, rechecks eligibility before delivery, and stops for enrolled authenticators, verified passkeys or user opt-out. Failed sends wait until the next weekly attempt and are not marked delivered. Project administrators must activate the setting and supply an HTTPS security-page URL from approved project origins. No reminder email has been sent and no live delivery schedule has been activated.

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
