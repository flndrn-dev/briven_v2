# Shared AI provider implementation plan

> Execute natively in this session under the owner's instruction to continue and accepted complete-AI design. Preserve the private stage and cutover gate.

**Goal:** Connect every existing Briven AI feature through a consistent, tested provider interface that can use Ollama or xAI.

**Architecture:** A dependency-injected transport owns provider authentication, bounded output, parsing and streaming. A small environment wrapper selects provider/model and is consumed by generation, Studio and support services. Provider output remains untrusted; database changes continue through validated Studio operations.

**Tech stack:** Bun, TypeScript, fetch, existing Hono and Zod.

**Spec:** `docs/superpowers/specs/2026-10-06-briven-complete-ai-design.md`, model connection slice.

## Constraints

No new provider is activated without explicit selection and credentials. Default remains Ollama. xAI uses the fixed HTTPS `https://api.x.ai/v1/chat/completions` endpoint and Bearer authentication; redirects are rejected. Prompts, provider bodies and keys are not logged. No automatic retries or fallback with duplicate billable generations. Bound each response to 1 MiB, each stream frame to 256 KiB, and xAI generated output to 4096 tokens. Cancellation must abort the upstream connection. Empty, malformed, refused or truncated responses fail honestly.

## Review focus

Provider credentials must never enter user-visible errors. Existing feature-specific Ollama models cannot be sent to xAI. Arbitrarily split UTF-8/CRLF stream frames must parse correctly. Disconnect must stop upstream work. Provider selection must cover Studio and grounded support as well as the three generators.

## Task 1: Provider transport

Files: create `apps/api/src/services/ai-provider.ts` and `ai-provider.test.ts` in the website repository.
Interface: `createAiProvider(config, fetcher)` returns `complete(messages, options): Promise<string>` and `stream(messages, options): AsyncGenerator<string>`. Options carry model, temperature, JSON mode and timeout; configuration is explicitly discriminated by provider.

- [ ] Write wire-level tests using mocked fetch: both provider contracts, JSON mode, empty/truncated/error results, secret redaction, split Unicode/SSE frames, missing terminator and cancellation.
- [ ] Run failing tests, implement the bounded transport, then run all transport tests.
- [ ] Typecheck API and commit the tested transport.

## Task 2: Connect existing clients

Files: create `ai-client.ts`; modify API `env.ts`, services `ai-schema-gen.ts`, `ai-function-gen.ts`, `ai-explain.ts`, `ai-stream.ts`, `ollama.ts`, `mcp-answer-writer.ts`, route `ai.ts`, and `infra/dokploy/compose.serverless-postgres.yml`.
Interface: `aiConfigured()`, `aiModel(feature)`, `completeAi(messages, options)` and `streamAi(messages, options)` delegate to the transport; export one `AiNotConfiguredError` with backward-compatible generator import.

- [ ] Configure `BRIVEN_AI_PROVIDER` (ollama/xai), `BRIVEN_XAI_API_KEY`, `BRIVEN_XAI_MODEL`, preserve existing Ollama feature overrides and legacy Studio variables.
- [ ] Replace duplicated generation/chat requests and adapt token streaming to existing browser SSE events; make cancellation propagate upstream and errors customer-readable.
- [ ] Update explanation instructions to support PostgreSQL and pgvector as well as Briven functions. Keep grounded support strict.
- [ ] Verify transport and affected feature tests, API typecheck, compose syntax/log caps, and absence of obsolete direct provider calls; commit and push only Briven changes.

## Task 3: Live acceptance

- [ ] Obtain a working existing gateway or owner-provided funded xAI key through the private credential file; never print it.
- [ ] Publish targeted API image through CI, deploy privately, prove real schema/function/explanation, Studio plan and streaming against the owner acceptance project.
- [ ] Re-run affected core acceptance. Record provider failures honestly; retain the website cutover hold until the expanded release criteria are met.
