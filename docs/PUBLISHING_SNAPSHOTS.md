# Publishing bootstrap snapshots

Amaru nodes bootstrap from three consecutive epoch snapshots, downloaded from
a public R2 bucket and listed in
`crates/amaru/config/bootstrap/<network>/snapshots.json`. See
[BOOTSTRAP.md](./BOOTSTRAP.md) for how the snapshots themselves are built.

## Publishing new snapshots

1. Go to Actions → "Publish Bootstrap Snapshots" → "Run workflow".
2. Pick a network (`preprod`, `preview`, `mainnet` or `all`). Leave `epoch`
   empty to target the last three completed epochs, or set it to the first
   epoch of the set (the workflow publishes `epoch`, `epoch + 1` and
   `epoch + 2`).
3. The run opens a pull request updating `snapshots.json`. Review and merge.

The PR is created with `GITHUB_TOKEN`. This needs a one-time repository
setting: Settings → Actions → General → Workflow permissions → enable "Allow
GitHub Actions to create and approve pull requests".

`GITHUB_TOKEN` cannot trigger `pull_request` workflows, so the workflow
dispatches CI on the branch itself. To re-run CI, close and reopen the PR.

## What the workflow does

For each selected network:

1. `amaru create-snapshots`: downloads chain data from Mithril, replays it
   with `db-analyser` and packages the three epoch snapshots as tar.gz
   archives.
2. `scripts/publish-bootstrap-snapshots`: uploads the archives to the bucket
   (skipping any already there), checks they are publicly reachable and
   rewrites `snapshots.json`.
3. Runs `amaru bootstrap` from a scratch directory against the new manifest,
   exercising the same path end users take. Skip with `skip_verification`.
4. Opens a pull request with the manifest change.

Chain data and ledger snapshots stay on the runner between runs, so only the
first run is slow. Re-running for an already published epoch is safe: uploads
are skipped and no PR is opened when nothing changed.

## Runner setup

The workflow needs a self-hosted runner with the labels `self-hosted` and
`snapshots`.

- Linux or macOS, x86_64 or arm64.
- Disk: about 50 GB per testnet, 500 GB for mainnet.
- On `$PATH`: `rustup`, `aws`, `jq`, `curl`, `gh`, `git`.
- `db-analyser` is downloaded automatically (pinned by `CARDANO_NODE_VERSION`
  in the workflow, checksum-verified). To use your own build, put it on
  `$PATH` instead.

If the runner runs as a service, make sure those tools are on the service's
`PATH` (the runner reads a `.path` file from its directory).

Work files live in `$HOME/amaru-snapshots/<network>/`, or under the
`SNAPSHOTS_CACHE_DIR` repository variable when set:

- `dist/`: chain data and db-analyser ledger snapshots. Keep it; this is what
  makes runs incremental.
- `snapshots/`: generated archives. Old epochs can be deleted once published.

### Secrets and variables

| Name | Kind | Purpose |
|------|------|---------|
| `R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY` | secret | R2 credentials with write access to the bucket |
| `S3_ENDPOINT` | secret | R2 S3 endpoint |
| `SNAPSHOTS_BUCKET_NAME` | secret | Bucket the archives are uploaded to |
| `SNAPSHOTS_CACHE_DIR` | variable, optional | Cache directory on the runner |
| `SNAPSHOTS_PUBLIC_URL_BASE` | variable, optional | Public URL base; defaults to the base of the existing URLs in `snapshots.json` |

### Security

Amaru is a public repository, so lock the runner down:

- Put it in a runner group restricted to this repository.
- Keep this workflow `workflow_dispatch`-only and never add `pull_request`
  triggers to workflows targeting the `snapshots` label.
- Require approval for workflow runs from outside collaborators.

## Testing in a fork

1. Push the branch and get the workflow file onto the fork's default branch.
2. Create a bucket with public read access and set the secrets above.
3. Set `SNAPSHOTS_PUBLIC_URL_BASE` to the bucket's public URL. Without it the
   publish script uses the upstream URLs from `snapshots.json` and skips your
   uploads, since those epochs are already published upstream.
4. Register a runner with the `snapshots` label.
5. Run the workflow for `preview`, the smallest network.

## Running it by hand

The workflow is a thin wrapper; the same steps work on any machine meeting
the requirements above:

```shell
cargo run --release --bin amaru -- create-snapshots --network preprod -f

AWS_ACCESS_KEY_ID=... \
AWS_SECRET_ACCESS_KEY=... \
AWS_DEFAULT_REGION=auto \
BUCKET_NAME=... \
ENDPOINT=... \
AMARU_NETWORK=preprod \
bash ./scripts/publish-bootstrap-snapshots
```

Then commit the `snapshots.json` change and open a PR.
