# Briven gap closure implementation plan

**Goal:** Continue the ten requested areas through implementation and measured acceptance.
**Spec:** Existing complete-AI design and serverless cutover specifications. Execute natively under the owner's instruction to continue without routine confirmation.

## Order and acceptance

- [x] Finish account-agent conversation persistence, authenticated routes and dashboard. Reuse scoped services, enforce exact one-use write previews, test concurrent requests, removed membership, cross-tenant reads, and uncertain results using isolated PostgreSQL. Local verification passed; live acceptance is the next task.
- [ ] Publish API and dashboard images through existing CI; deploy to the verified private Dokploy stage. Prove real provider generation/streaming and account workflows with owner credentials kept outside source.
- [ ] Deploy the existing monitoring collector and dashboard; record real observations and warming-up baselines independently of model availability.
- [ ] Implement a fixed-playbook recovery executor and persisted attempt/lock history. Register only a worker whose controlled staging drill passes; retain the approved limits and escalation policy.
- [ ] Complete customer restore to a new isolated branch, preserving the original; prove branch contents and customer permission checks.
- [ ] Implement idle sleep and bounded compute resource control with live demand/idle trials before activation.
- [ ] Finish customer Auth enrollment from the knowledge-base requirements and existing isolated implementation; pass real project acceptance before enabling it.
- [ ] Re-run release acceptance and switch website domains only when the saved working-AI and compatibility gates pass. Preserve the old stack for rollback.

## Constraints

Preserve unrelated edits and existing data. Do not claim percentages as proof. Secrets stay in ignored private files or the deployment environment. Existing confirmed account actions remain a product security requirement even though development does not need repeated owner approval. No production recovery registration without a successful real staging drill, no automatic database/host restart, no added infrastructure spending, and no general-purpose model SQL/shell execution.
