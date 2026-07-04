# Sphare

[Sphare](https://www.sphare.space) is a web application providing community created and managed forums to exchange with other people about your hobbies, news, art, jokes and many more topics.

Sphare is an ad-free, open source website with a focus on transparency, privacy and community empowerment.
Sphare is built in Rust using [Leptos](https://github.com/leptos-rs).

## Why "Sphare"?

Sphare, pronounced like S-fair ([sfɛr]) or "Sphère" in French, is the combination of Sphere, the communities on Sphare, and Share.

The name symbolizes this project's goals: to enable people to discuss their interests and share knowledge. At the same time, it still acknowledges the "bubbles" we often build for ourselves and lets you access other viewpoints without control from an algorithm. By adhering to these principles and relying on donations rather than ads, Sphare aims to provide a healthier alternative to Big Tech social media.

## License

This project is licensed under the terms of the GNU Affero General Public License version 3 (AGPLv3).  

See [`LICENSE`](./LICENSE) for the full text.

## Setting up Sphare

### Installing the Rust toolchain

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. `rustup toolchain install nightly --allow-downgrade` - make sure you have Rust nightly
3. `rustup target add wasm32-unknown-unknown` - add the ability to compile Rust to WebAssembly
4. `cargo install cargo-leptos` - install the `cargo-leptos` binary
5. `cargo install sqlx-cli --no-default-features --features rustls,postgres` - Install sqlx-cli

### Install TailwindCSS and DaisyUI

1. [Install npm](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm)
2. `npm install` - install `TailwindCSS` and `DaisyUI`

### Setup Postgres DB and Keycloak

To build and run Sphare, you will need a Postgres database and a Keycloak realm with a client for Sphare. You can find a docker-compose file to start them in the `setup` folder. Please note that the docker-compose file creates a database for Keycloak but you should create a separate one for Sphare itself.

1. Create a Postgres DB and Keycloak instance, for instance using `setup/docker-compose.yml` with `podman-compose up` inside the `setup` folder.
2. Create a Sphare realm in Keycloak, you can import the `dev-sphare-realm-export-2026-04-12.json` for a quicker setup.
   * Don't forget to reset the password of the `sphare-app` client in the `sphare` realm.
3. Add a `.env` file in the repo's root folder with your Postgres connection, e.g. `DATABASE_URL=postgres://<user>:<password>@<postgres_url>/<schema_name>`
4. Run `sqlx migrate run` - perform migrations on the DB 
5. Set the following environment variables:
    * OIDC_ISSUER_ADDR - url of the keycloak instance
    * AUTH_CLIENT_ID - ID of the Sphare client in Keycloak
    * AUTH_CLIENT_SECRET - Secret of the Sphare client in Keycloak
    * DATABASE_URL - Postgres database url
    * TEST_DATABASE_URL - Prefix of the test database url for the integration tests in the form of postgres://(user):(pwd)@(ip address):(port)/ 
    * TEST_DATABASE_NAME - Name of the root test database (will be appended to TEST_DATABASE_URL), the integration tests will connect to this database and create new databases to run each test in isolation.
    * SESSION_KEY - Key to persist session data
    * SESSION_DB_KEY - DB key to persist session data
    * TEST_DATABASE_URL - Test DB url, used in integration tests
    * LEPTOS_ENV - Used to set some headers, use "DEV" for a development environment

To populate your development database, you can run `cargo test populate_dev_db -- --ignored` which will create some Spheres, posts and comments in your database (given by DATABASE_URL).

### Additional environment variables

If you want to store icons and banners for Spheres, you can configure an S3 storage with the following environment variables:
* AWS_ACCESS_KEY_ID
* AWS_SECRET_ACCESS_KEY
* AWS_ENDPOINT
* OBJECT_CONTAINER_URL
* ICON_BUCKET
* BANNER_BUCKET

## Running Sphare

```bash
cargo leptos watch
```

## Compiling for Release
```bash
cargo leptos build --release
```

Will generate your server binary in target/server/release and your site package in target/site

## Testing

### Unit & integration tests
```bash
cargo test
```

### End-to-end tests
Run `npm install` in the end2end subdirectory before testing
```bash
cargo leptos end-to-end
```

```bash
cargo leptos end-to-end --release
```

Cargo-leptos uses Playwright as the end-to-end test tool.  
Tests are located in end2end/tests directory.

## Executing a Server on a Remote Machine Without the Toolchain
After running a `cargo leptos build --release` the minimum files needed are:

1. The server binary located in `target/release/server`
2. The `site` directory and all files within located in `target/site`

Copy these files to your remote server. The directory structure should be:
```text
start-axum
site/
```
Set the following environment variables (updating for your project as needed):
```text
LEPTOS_OUTPUT_NAME="start-axum"
LEPTOS_SITE_ROOT="site"
LEPTOS_SITE_PKG_DIR="pkg"
LEPTOS_SITE_ADDR="127.0.0.1:3000"
LEPTOS_RELOAD_PORT="3001"
```
Finally, run the server binary.
