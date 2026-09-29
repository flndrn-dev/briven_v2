# Customer database home and create-a-project connection

Date: 2026-09-29
Status: agreed 2026-09-29. The local build steps are in docs/superpowers/plans/2026-09-29-customer-database-home.md. That plan does not install the engine or switch the live site.

This file records where a customer's database lives, and how Create a project connects to it. It follows the Sprint 3 cutover in `2026-09-28-sprint-3-serverless-cutover-design.md`. It does not deploy anything.

## Decision

Customer databases live on the Briven engine, on the same computer that already serves briven.tech (`187.77.183.190`), in a private back room. The website stays the public shop window.

Notes have called that computer both KVM2 and KVM4. The choice is that one computer, the one the public site already uses. It is not a second machine, and it is not an outside database company. Before any install, read the server name in the Dokploy panel and use the entry that already runs the website. Do not guess between the two names.

## Who this is for

A signed-in member of a team who creates a project, and later connects their own app to that project's database.

## Who this is not for

- Copying old accounts, projects, or customer rows onto the new databases.
- A visitor who is not signed in.
- One shared database, or one master key, for every customer.
- Putting the engine's master key on the public website.
- Switching the live site before the private test below has passed and a separate go-ahead is given.

## Rooms

The website keeps sign-in, teams, billing, and control keys in its own PostgreSQL control database.

The engine keeps one customer database per website project, plus that project's history line and its running database. The website asks the engine only with a signed hall pass that lasts 60 seconds. The pass names the team and the project. The engine's master key never crosses the public website.

The engine's private door is reachable from the website on that computer. It is not a public address. The engine shape from the Sprint 3 design still applies. In this repo those parts are the storage controller, pageserver, at least three safekeepers, durable object storage, a compute scheduler, the private control API, and a TLS Postgres proxy. Metadata stays in a separate PostgreSQL catalog with versioned migrations, backups, and a restore check.

## Create a project

1. The person is signed in and submits a project name.
2. The website checks the name. A name that is already finished for that team is refused. The existing database stays the one they have.
3. The website knocks with the 60-second hall pass.
4. The engine starts one empty database. If an earlier try for that same name is unfinished or failed, it continues that same database and does not create a second one.
5. The engine turns on the similar-things search (`pgvector`) and runs a small vector query.
6. The project becomes ready only after that query works.
7. If the query fails, the project is marked failed and the same database is kept. Create can be pressed again.
8. Two teams may use the same project name. One team may not have two databases with that name.

A finished name returns a conflict. An unfinished or failed name returns the same project. If two creates race and both try to insert, the loser gets a conflict and does not take over the other row.

## The customer's own key

When the project is ready, the website can give the customer a key for that database only.

- The key is random.
- It lasts 15 minutes, the same lifetime as the website's current shell key.
- It opens only that project's database.
- It travels through a locked door, the same kind of lock a bank website uses.
- The engine's master login is refused on that door.
- A person who can only look, or whose pass was taken away, cannot create a project and cannot receive a key that writes.
- A key made for one project fails on another project, even if the address is changed.

The website asks the engine for this key. The website does not store the engine master key, and the key is not written into the project files. Real passwords stay in the Dokploy panel.

## What we check in private

On the briven.tech computer, before the public site points at this door:

- A signed-in person creates a project and gets one empty database.
- The project stays unfinished until the similar-things query works.
- Stopping the engine in the middle of create, then pressing create again, keeps one database.
- A look-only person, and a person whose pass was removed, cannot create and cannot get a write key.
- A key from one project fails on another project.
- A failed catalog migration leaves the catalog at the previous version.
- A row written with the project's own key is still there after the private door and that database are restarted.

The public website stays on its current setup until that test passes. Old customer data stays where it is. Saved-copy and undo buttons stay off. Passwords stay out of git.

## Out of scope

- Building real restore from engine branches. Saved copies stay unavailable.
- Publishing the practice compose or any sample password.
- Importing the old production volumes. Those volumes stay intact for rollback.
- Hosting an AI model. The similar-things search is the database feature. The model service is a later product.
- A public cutover. That needs the private test above and a separate go-ahead.
