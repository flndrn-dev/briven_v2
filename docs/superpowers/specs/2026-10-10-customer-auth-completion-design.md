# Customer Auth completion design

**Status:** Proposed for owner review. No new recovery policy or Auth implementation is approved by this document alone.

**Scope:** Complete shared Briven Auth on the accepted serverless PostgreSQL rebuild. Preserve optional extra protection, tenant isolation and weekly reminders defaulting off. This does not replace the current dashboard Better Auth or customer Briven FDI implementation, migrate old customer data, or introduce application-specific payment rules.

## Human description

A customer can keep ordinary login or choose extra protection. Authenticator setup shows a QR and a manual key, checks a real code, then asks the customer to save emergency codes. Once protection is enabled, another login method must not silently bypass it. A verified passkey can supply strong authentication without requiring the same person to also enter an authenticator code.

An emergency code works once. It opens only the recovery screen, where the customer replaces the lost authenticator, verifies the replacement and saves new codes. It does not unlock the application before recovery is finished.

If every device and emergency code is lost, email access alone cannot remove the protection. The recommended workflow requires two different project owners to review identity through the organization's established independent channels, verify their own accounts again and approve one short recovery opportunity. A project with only one eligible owner cannot use this reset. This recommendation needs the owner's approval before implementation.

An application can ask for identity to be checked again for a particular action. Its backend checks the result with Briven, including the account, project, action and expiry. The browser cannot declare itself verified, and Briven does not decide which business actions need this check.

## Approach

Recommended: extend the existing project-scoped FDI, native sessions, enrollment and transactional proof services. This preserves the accepted architecture and existing customer integration.

A UI-only completion would leave recovery and factor bypasses in the API. Replacing Auth with another provider would add migration and operating scope outside this task. Neither alternative meets the current gap-closure objective.

The July knowledge base describes the older Doltgres architecture. The accepted October serverless rebuild uses its existing PostgreSQL-native Auth pool; this design does not change that storage architecture. The referenced local `AI_DOCS/dolt-reference/00-doltgres-truth.md` is missing and is not evidence of running services.

## Account and enrollment

- Add a project-scoped FDI session/account endpoint using `requireFdiProjectKey` and `verifyAuthCoreSession(expectedTenantId)`. Return only the needed user, session expiry and protection/recovery state, with `Cache-Control: no-store`. Bind user lookup to both tenant and user.
- Replace the hosted account page's retired `/v1/auth-tenant/get-session` and sign-out paths with the active FDI account and sign-out routes. Recovery sessions render only recovery controls; normal sessions render account security.
- Show a locally rendered QR from the existing `otpauthUrl`, a copyable manual secret and a numeric code field. Never send the secret to a third-party QR service or include it in telemetry. Preserve the existing pending-device resume behavior.
- Enable the device only after successful existing TOTP verification. Return newly generated emergency codes once. Store only hashes and a versioned saved-confirmation record. Confirmation must identify the exact current code-set generation; stale confirmation is rejected. The UI explains that a refresh cannot reveal lost codes again and offers regeneration only after fresh identity verification.
- Enrollment completion includes explicit acknowledgement that the current emergency-code set was saved. Closing the screen does not invent that acknowledgement or reverse an already verified authenticator.
- Preserve project branding and the existing hosted layout. Weekly reminders remain disabled until real sender and approved HTTPS security-page acceptance; users can opt out.

## Sessions and login consistency

- Apply one shared post-primary-login policy to password, email-code, magic-link, social and SSO completion. A user with a verified authenticator receives the existing bounded, transactional MFA challenge before a normal session is created. Users without enrollment keep ordinary authentication.
- A cryptographically verified passkey with user verification satisfies strong authentication; record its method and verification time. Browser claims, a credential ID alone or a provider login label cannot supply that assurance.
- Preserve existing retry limits, single-use challenge/proof consumption and tenant binding. A wrong code must not require repeating primary login until the bounded challenge attempts are exhausted.
- Add a normal-session guard rejecting `recoveryRequired` sessions on application, credential-management and fresh-action verification routes. Recovery-only endpoints explicitly opt into recovery sessions and check tenant/user binding.
- Held/archived-user and session-store errors fail closed. A migration or unavailable column is a deployment readiness problem, not permission to bypass moderation or recovery checks.

## Emergency-code recovery

- Preserve the existing 15-minute recovery session (`ttlDays: 1/96`) and non-refreshable behavior. Restrict it to recovery state, replacement authenticator setup/verification, code-set confirmation and sign-out.
- Use an explicit recovery transition that binds the new pending authenticator to the same tenant/user/recovery session. It cannot delete the last protection method or change the user identifier before replacement proof succeeds.
- Commit replacement proof, previous authenticator invalidation, new hashed emergency-code generation and recovery progress atomically. Normal session issuance requires confirmation of the new code-set generation; revoke the recovery session and older sessions at completion. Interrupted responses reconcile the same progress instead of consuming another emergency code or making another device.
- Test expiry, replay, simultaneous code submissions, cancelled setup, interrupted completion and cross-project requests against real disposable PostgreSQL.

## Lost all devices and emergency codes: proposed owner review

- Offer a recovery request after a valid existing primary-login proof. Requests expose only pending/approved/denied/expired state to their bound user and project. Rate-limit and expire requests after 24 hours; do not send a login or reset grant by email.
- Require approval from two different currently authorized project owners, each using the dashboard's normal fresh password verification within five minutes. These are two independent approvals, not a claim that the dashboard already has a second-factor enrollment service. Developers, viewers, general API keys and support staff without project-owner authority cannot approve. Both approvals must still be authorized and no older than ten minutes when the grant is created.
- Require a recorded attestation that identity was checked through the organization's established records and an independent trusted contact channel. Store a case/reference identifier and decision, not identity documents, answers or other sensitive evidence. Briven enforces authority, proof freshness and the audit trail; the project owner is accountable for the external identity check.
- The second approval atomically creates one hashed, one-use recovery grant lasting ten minutes, bound to the request, tenant and user. Concurrent approval cannot produce duplicate grants. Redemption also requires a fresh primary-login proof for that same user and leads only to the restricted replacement flow above.
- Revoke old sessions and protection credentials only when the verified replacement is committed. Approval alone must not remove protection. Denial, expiry and cancellation leave it intact.
- A project lacking two eligible owners must not receive an email-only or automatic platform bypass. Its recovery request stays unavailable with a clear support explanation; changing this authority model requires a separate explicit policy.

This is a proposed policy decision, not evidence of an implemented or tested recovery service.

## Stable passkey settings and legacy secrets

- Persist the selected RP ID as project configuration, validate it against approved HTTPS origins and lock it once a credential has been issued. Browser parameters can select an approved origin but cannot change the stored RP ID.
- Changing a configured origin must preserve the RP relationship and must not silently orphan existing credentials. An RP move requires a separate visible migration/re-enrollment workflow; this release rejects an incompatible edit.
- Keep discoverable credentials and user verification required. Validate signatures, challenge, expected origin, RP hash, user binding and credential counters transactionally.
- Add an idempotent, bounded legacy-secret sweep using the existing encryption envelope/key. Upgrade only positively identified legacy plaintext rows; malformed envelopes, missing keys and decryption errors block release without logging secrets. Do not replace or rotate keys as part of the sweep. Counts and remaining legacy rows are recorded for acceptance.

## Fresh action verification contract

- Use an opaque, hashed, one-use proof backed by PostgreSQL, not a browser-only flag. Bind the challenge/proof to project, tenant, live session, user, action name, immutable operation ID and a SHA-256 payload digest supplied by the application backend.
- The application backend creates challenges through a project-scoped route authorized by its existing non-public `brk_` admin key. Publishable `pk_briven_auth_` keys and browser-only sessions cannot create or consume backend action proofs. Never mint a live backend key implicitly.
- The customer completes the challenge through the existing first-party FDI path using a verified authenticator or a verified passkey. For users without extra protection, an enabled primary method can reverify the same account; an enrolled factor cannot be bypassed by choosing password or email alone. Emergency codes and recovery sessions cannot authorize sensitive actions.
- Challenges expire after five minutes. Successful proof expires after 60 seconds. The backend may request a maximum verification age from 30 to 300 seconds, defaulting to 300; Briven enforces the stricter proof/session limits.
- Consume checks current project access/key validity, session validity, moderation/recovery state, expected operation and payload digest, proof expiry and one-use state in one transaction. Concurrent consumes produce one success. A revoked session or unavailable store refuses verification.
- Changed retry payloads return conflict. A timed-out consume must not automatically execute or replay the business action. Applications retain their own durable action idempotency and reconcile their action status; the SDK documents this explicitly.
- Add typed browser enrollment/recovery methods and server-only challenge/consume helpers. Browser bundles must not include backend keys, database credentials or privileged imports. No payment names, fixed app domains or automatic business policies enter the platform contract.

## Acceptance and release

Automated acceptance covers real PostgreSQL concurrency, strict project/session isolation, login-method consistency, recovery route restrictions, expiry and replay, legacy sweep repeatability, immutable RP settings and proof consumption. API/SDK/web typechecks, focused lint and production builds must pass.

Private staged acceptance uses actual owner/viewer/foreign-project sessions and normal recent verification. Owner device acceptance must include QR scan and code entry on an actual phone, manual-key setup, passkey enrollment and sign-in, cancellation, cross-device behavior, emergency-code recovery and fresh action verification. Synthetic WebAuthn responses and browser phone emulation do not satisfy these device gates.

The final matching recovery-worker drill follows the last backend build. Paired credential rotation/revocation remains the final handover step as the owner requested. Public cutover follows accepted private behavior and compatibility checks; no 100% production claim precedes those gates.

## Sources checked

Project `related-repositories/briven-website/AGENTS.md`, `docs/knowledge-base.md` including its Doltgres section, the actual mounted FDI/session/enrollment services, and `2026-10-05-briven-auth-readiness.md` were reviewed.

- [SuperTokens TOTP setup](https://supertokens.com/docs/additional-verification/mfa/totp/totp-for-all-users): show enrollment information and validate a code before device verification. Briven keeps the owner's optional-enrollment policy.
- [Recovery codes](https://supertokens.com/docs/additional-verification/mfa/backup-codes): recovery codes are a distinct fallback factor and need backend enforcement.
- [Step-up authentication](https://supertokens.com/docs/additional-verification/mfa/step-up-auth): protected backend routes must enforce the required verification.
- [Passkey concepts](https://supertokens.com/docs/authentication/passkeys/important-concepts): credentials belong to a relying party and may be synced or device-bound; actual device acceptance remains required.

The proposed owner-review authority, exact action-proof contract and expiry values are Briven design choices for review, not claims that these references mandate them.
