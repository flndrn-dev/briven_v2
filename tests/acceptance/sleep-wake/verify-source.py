"""Bind a changing acceptance harness to the unchanged, repaired application."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
CI_BLOCK = '''# Explicit CI target: never copied into the runtime image. A unique run value
# ensures the real private PostgreSQL test executes on every acceptance run.
FROM build AS lifecycle-acceptance
ARG GIT_VERSION
ARG BRIVEN_ACCEPTANCE_RUN
COPY --from=lifecycle-harness rust-postgres-acceptance.sh /tmp/briven-rust-postgres-acceptance.sh
RUN /bin/bash /tmp/briven-rust-postgres-acceptance.sh

'''


def runtime_recipe(text):
    assert text.count(CI_BLOCK) == 1, 'Unexpected test-stage recipe; do not mask runtime changes'
    return text.replace(CI_BLOCK, '', 1)


def main():
    assert os.environ.get('GITHUB_ACTIONS') == 'true'
    source, harness = os.environ['BRIVEN_ACCEPTANCE_ENGINE_SOURCE'], os.environ['GITHUB_SHA']
    assert re.fullmatch('[0-9a-f]{40}', source) and re.fullmatch('[0-9a-f]{40}', harness)
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=ROOT, text=True).strip()
    assert git('rev-parse', 'HEAD') == harness
    changed = git('diff', '--name-only', source, harness).splitlines()
    assert all(name == 'Dockerfile' or name.startswith(('tests/acceptance/', '.github/workflows/sleep-wake-', 'docs/')) for name in changed), 'Application inputs changed; build a new application revision'
    original = subprocess.check_output(['git', 'show', source + ':Dockerfile'], cwd=ROOT, text=True)
    normalized = runtime_recipe((ROOT / 'Dockerfile').read_text())
    assert normalized == original, 'Runtime Docker instructions changed'
    ignore = subprocess.check_output(['git', 'show', source + ':.dockerignore'], cwd=ROOT)
    assert ignore == (ROOT / '.dockerignore').read_bytes()
    proof = {'phase': 'sleep-wake-source-proof', 'passed': True,
             'engineSource': source, 'harnessSource': harness, 'runtimeEquivalent': True,
             'runtimeRecipeHash': hashlib.sha256(normalized.encode()).hexdigest(),
             'dockerignoreHash': hashlib.sha256(ignore).hexdigest(), 'changedPaths': changed}
    path = Path(os.environ['RUNNER_TEMP']) / 'sleep-wake-source-proof.json'
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, 'w') as output: json.dump(proof, output)
    print(json.dumps(proof), flush=True)


if __name__ == '__main__': main()
