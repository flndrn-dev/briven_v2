# Docker images of Briven

## Images

Briven v2 uses the Neon-derived engine and builds two main image families:

- `ghcr.io/flndrn-dev/briven-engine` — image with pre-built `pageserver`, `safekeeper`, `storage_broker`, and `proxy` binaries plus runtime dependencies. Built from [/Dockerfile](/Dockerfile).
- `ghcr.io/flndrn-dev/briven-compute-node-v16` — compute node image with pre-built Postgres binaries from the Neon-derived Postgres fork. Similar images exist for v17, v15, and v14. Built from [/compute-node/Dockerfile](/compute/compute-node.Dockerfile).

The upstream Neon image names are intentionally not removed from source code until Briven's own registry, release workflow, and compatibility tests are in place. Use Docker variables to point examples at upstream images when comparing against Neon.

## Build pipeline

The Briven image naming target is:

1. `ghcr.io/flndrn-dev/briven-compute-node-v17` (and -16, -v15, -v14)

2. `ghcr.io/flndrn-dev/briven-engine`

3. `ghcr.io/flndrn-dev/briven-test-extensions-v17` (and -16, -v15, -v14)

## Docker Compose example

You can see a [docker compose](https://docs.docker.com/compose/) example to create a local Briven engine cluster in [/docker-compose/docker-compose.yml](/docker-compose/docker-compose.yml). It creates the following containers.

- pageserver x 1
- safekeeper x 3
- storage_broker x 1
- compute x 1
- MinIO x 1        # This is Amazon S3 compatible object storage

### How to use

1. create containers

You can specify the version and image source using the following environment values.

- PG_VERSION: postgres version for compute (default is 16 as of this writing)
- BRIVEN_ENGINE_IMAGE: engine image name. Default is `ghcr.io/flndrn-dev/briven-engine`
- BRIVEN_COMPUTE_REPOSITORY: compute image repository. Default is `ghcr.io/flndrn-dev`
- BRIVEN_COMPUTE_IMAGE: compute image basename. Default is `briven-compute-node-v${PG_VERSION}`
- TAG: image tag. Default is `latest`

```
$ cd docker-compose/
$ docker-compose down   # remove the containers if exists
$ PG_VERSION=16 TAG=latest docker-compose up --build -d  # You can specify the postgres and image version
Creating network "dockercompose_default" with the default driver
Creating docker-compose_storage_broker_1       ... done
(...omit...)
```

2. connect compute node
```
$ psql postgresql://cloud_admin:cloud_admin@localhost:55433/postgres
psql (16.3)
Type "help" for help.

postgres=# CREATE TABLE t(key int primary key, value text);
CREATE TABLE
postgres=# insert into t values(1, 1);
INSERT 0 1
postgres=# select * from t;
 key | value
-----+-------
   1 | 1
(1 row)

```

3. If you want to see the log, you can use `docker-compose logs` command.
```
# check the container name you want to see
$ docker ps
CONTAINER ID   IMAGE                                              COMMAND                  CREATED         STATUS         PORTS                                                                                      NAMES
3582f6d76227   docker-compose_compute                             "/shell/compute.sh"      2 minutes ago   Up 2 minutes   0.0.0.0:3080->3080/tcp, :::3080->3080/tcp, 0.0.0.0:55433->55433/tcp, :::55433->55433/tcp   docker-compose_compute_1
(...omit...)

$ docker logs -f docker-compose_compute_1
2022-10-21 06:15:48.757 GMT [56] LOG:  connection authorized: user=cloud_admin database=postgres application_name=psql
2022-10-21 06:17:00.307 GMT [56] LOG:  [NEON_SMGR] libpagestore: connected to 'host=pageserver port=6400'
(...omit...)
```

4. If you want to see durable data in MinIO which is s3 compatible storage

Access http://localhost:9001 and sign in.

- Username: `minio`
- Password: `password`

You can see durable pages and WAL data in the `briven` bucket.
