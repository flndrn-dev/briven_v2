# Briven website and engine deployment design

## Goal

Deploy Briven's website/dashboard and the Briven v2 database engine from separate GitHub repositories, with each Dokploy deployment following pushes to that repository's `main` branch. The website sign-in page must support email/password and email one-time-code sign-in.

## Repository and deployment boundaries

- Keep `https://github.com/flndrn-dev/briven_v2.git` as the engine repository and preserve its existing root layout.
- Create a private `https://github.com/flndrn-dev/briven-website.git` repository from the current Briven web-platform source, including the website, dashboard, API, and deployment definitions those features depend on. Exclude local environment files and test private keys.
- Configure two independent Dokploy projects. The website project tracks `briven-website:main` and uses the Dokploy compose definition in `infra/dokploy/compose.dokploy.yml`. The engine project tracks `briven_v2:main` and uses an engine-specific compose definition. Both projects must have GitHub auto-deploy enabled so pushes to their own main branches trigger their own deployment.
- Keep website and engine health checks, runtime environment, and deploy status independent. Do not copy website runtime secrets into either repository.
- The engine repository currently includes a local development stack. Before using it as a public production database endpoint, verify persistent storage, secret configuration, network exposure, compute connectivity, and backup/restore behavior. The deployment must not expose its sample credentials or MinIO console publicly.
- The current website/dashboard stack has its own API and DoltGres data plane. The v2 engine's hosted customer control plane is still open work, so this deployment keeps the v2 engine isolated; it does not silently switch dashboard projects to the v2 engine. Connect them only after the hosted API, roles, TLS routing, and recovery work is complete.

## Sign-in behavior

- Keep email/password sign-in using the existing Better Auth configuration and password policy.
- Add Better Auth's email OTP plugin to the platform's main auth instance, generate a six-character mixed uppercase-letter-and-digit code with a cryptographically secure random source, expire it after five minutes, and limit verification attempts to three.
- Send sign-in codes using Briven's existing email delivery path. OTP sign-in must obey the same account-creation gate as password and magic-link sign-in, so closed signups cannot create accounts by requesting a code.
- Replace the website's magic-link form with a code-request and code-verification flow. Keep social-provider buttons when configured. Add clear pending, resend, expired/invalid-code, and success states without revealing whether an email address has an account.

## Verification

- Test the OTP generator's alphabet, length, and secure randomness contract, plus OTP request/verify API behavior, expiry, attempt limits, and signup policy.
- Verify the sign-in UI supports password and email-code flows, handles error and resend states, and preserves existing social sign-in behavior.
- Run the website production build and the focused auth tests.
- Validate both compose configurations and confirm Dokploy reports each deployment healthy after the corresponding GitHub push.

## Rollout

Create and push the private website repository first, connect the website Dokploy project with GitHub auto-deploy enabled, and verify the website/dashboard. Configure the engine project separately, validate its runtime requirements and private networking, then deploy it from `briven_v2`. Keep production credentials in Dokploy. Report each service's live URL and health independently.
