#!/usr/bin/env python3
"""Guard the release workflow's image order and concurrency key.

AUD-2026-11-16-05 found that the image job pushed the release tag before the
no-Python and image-contract checks ran, and that a tag push and a manual
dispatch for the same tag used different concurrency keys. This test fails when
either property returns: no step other than the final push may publish, the
push must come after both checks, and the concurrency key must be derived from
the release tag or the tag name.

The check is textual because the repository has no YAML dependency. It reads
only the workflow file and no network, clock, or external data.
"""

from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"
GROUP = "release-${{ inputs.release_tag || github.ref_name }}"
BUILD = "Build the image"
NO_PYTHON = "Assert the image carries no Python"
CONTRACT = "Check the image contract"
PUSH = "Push the verified image"
COMPOSED = "Run the composed suite on the local image"
CHECK_STEPS = (NO_PYTHON, CONTRACT)


def job(text: str, name: str, next_name: str) -> str:
    start = text.find(f"\n  {name}:\n")
    end = text.find(f"\n  {next_name}:\n", start + 1)
    if start == -1 or end == -1:
        return ""
    return text[start:end]


def image_job(text: str) -> str:
    block = job(text, "image", "quality")
    return block if block else job(text, "image", "publish")


def steps(text: str) -> dict[str, str]:
    found: dict[str, str] = {}
    for block in text.split("\n      - name: ")[1:]:
        name, _, body = block.partition("\n")
        found[name.strip()] = body
    return found


def problems(text: str) -> list[str]:
    found: list[str] = []
    group_line = None
    in_concurrency = False
    for line in text.splitlines():
        stripped = line.strip()
        if line.startswith("concurrency:"):
            in_concurrency = True
            continue
        if in_concurrency:
            if not stripped or stripped.startswith("#"):
                continue
            if stripped.startswith("group:"):
                group_line = stripped.split("group:", 1)[1].strip()
            break
    if group_line != GROUP:
        found.append(f"concurrency group is {group_line!r}, expected {GROUP!r}")

    image = image_job(text)
    if not image:
        return found + ["the image job is missing"]
    order = steps(image)
    for name in (BUILD, *CHECK_STEPS, PUSH):
        if name not in order:
            found.append(f"the image job has no {name!r} step")
    if any(name not in order for name in (BUILD, *CHECK_STEPS, PUSH)):
        return found

    for name in (BUILD, *CHECK_STEPS):
        if "docker push" in order[name]:
            found.append(f"{name!r} publishes the image before the checks pass")
    if "docker push" not in order[PUSH]:
        found.append(f"{PUSH!r} does not publish the image")
    names = list(order)
    push_at = names.index(PUSH)
    for name in CHECK_STEPS:
        if names.index(name) > push_at:
            found.append(f"{name!r} runs after the image is published")
    if COMPOSED not in order:
        found.append(f"the image job has no {COMPOSED!r} step")
    else:
        if "docker push" in order[COMPOSED]:
            found.append(f"{COMPOSED!r} publishes the image before the suite passes")
        if names.index(COMPOSED) > push_at:
            found.append(f"{COMPOSED!r} runs after the image is published")
        if "tools/test_tiers.sh composed" not in order[COMPOSED]:
            found.append(f"{COMPOSED!r} does not run the composed tier")

    if "Dockerfile.release" not in image:
        found.append("the image job does not use Dockerfile.release")
    if "gh run download" not in image or "release-binaries-$GITHUB_SHA" not in image:
        found.append("the image job does not download the exact-SHA release binaries")
    if "cargo build" in image:
        found.append("the image job compiles Rust instead of promoting prebuilt binaries")

    if "\n  desktop:\n" in text:
        found.append("the desktop job must be removed")
    quality = job(text, "quality", "publish")
    if not quality:
        found.append("the quality job is missing")
    else:
        for command in ("bun run check", "bun test", "bun run build", "bun run e2e"):
            if command not in quality:
                found.append(f"the quality job does not run {command!r}")
    cli = job(text, "cli", "publish")
    if not cli:
        found.append("the CLI release job is missing")
    elif "cargo test -p quantrule-shell" not in cli:
        found.append("the CLI release job does not test quantrule-shell")
    publish = text[text.find("\n  publish:\n") :]
    if "\n      - quality\n" not in publish:
        found.append("the publish job does not require the quality job")
    if "\n      - cli\n" not in publish:
        found.append("the publish job does not require the CLI job")
    if "if: steps.reference.outputs.publish == 'true'" not in order[PUSH]:
        found.append(f"{PUSH!r} is not limited to a publish run")
    if "docker push \"$IMAGE:$sha_tag\"" not in order[PUSH]:
        found.append(f"{PUSH!r} does not push the commit sha tag")
    return found


class ReleaseWorkflowTests(unittest.TestCase):
    def setUp(self) -> None:
        self.text = WORKFLOW.read_text(encoding="utf-8")

    def test_the_release_workflow_passes(self) -> None:
        self.assertEqual(problems(self.text), [])

    def test_the_old_concurrency_key_is_reported(self) -> None:
        seeded = self.text.replace(
            "${{ inputs.release_tag || github.ref_name }}",
            "${{ inputs.release_tag || github.ref }}",
            1,
        )
        self.assertTrue(
            any("concurrency group" in problem for problem in problems(seeded))
        )

    def test_a_push_during_the_build_is_reported(self) -> None:
        seeded = self.text.replace(
            '              .\n          else\n            docker build --platform linux/amd64 --file Dockerfile.release --tag "$REF" .',
            '              .\n            docker push "$REF"\n          else\n            docker build --platform linux/amd64 --file Dockerfile.release --tag "$REF" .',
            1,
        )
        self.assertNotEqual(seeded, self.text)
        self.assertTrue(
            any("before the checks pass" in problem for problem in problems(seeded))
        )

    def test_a_missing_check_is_reported(self) -> None:
        seeded = self.text.replace(f"- name: {NO_PYTHON}", "- name: Skipped check", 1)
        self.assertTrue(
            any("has no 'Assert the image carries no Python' step" in problem for problem in problems(seeded))
        )

    def test_a_missing_composed_step_is_reported(self) -> None:
        seeded = self.text.replace(f"- name: {COMPOSED}", "- name: Skipped composed", 1)
        self.assertTrue(
            any("has no 'Run the composed suite on the local image' step" in problem for problem in problems(seeded))
        )

    def test_a_missing_quality_job_is_reported(self) -> None:
        seeded = self.text.replace("\n  quality:\n", "\n  skipped-quality:\n", 1)
        self.assertTrue(
            any("quality job is missing" in problem for problem in problems(seeded))
        )

    def test_a_missing_release_binary_download_is_reported(self) -> None:
        seeded = self.text.replace(
            '          gh run download "${{ needs.gate.outputs.ci_run_id }}"',
            '          echo skipped',
            1,
        )
        self.assertNotEqual(seeded, self.text)
        self.assertTrue(
            any("exact-SHA release binaries" in problem for problem in problems(seeded))
        )

    def test_a_missing_cli_job_is_reported(self) -> None:
        seeded = self.text.replace("\n  cli:\n", "\n  skipped-cli:\n", 1)
        self.assertTrue(
            any("CLI release job is missing" in problem for problem in problems(seeded))
        )

    def test_a_desktop_job_is_reported(self) -> None:
        seeded = self.text.replace(
            "\n  publish:\n",
            "\n  desktop:\n    runs-on: ubuntu-latest\n\n  publish:\n",
            1,
        )
        self.assertNotEqual(seeded, self.text)
        self.assertTrue(any("desktop job must be removed" in problem for problem in problems(seeded)))

    def test_a_push_before_the_contract_check_is_reported(self) -> None:
        seeded = self.text.replace(
            "      - name: Check the image contract",
            "      - name: Push the verified image\n"
            "        if: steps.reference.outputs.publish == 'true'\n"
            "        run: |\n"
            '          docker push "$REF"\n'
            '          docker push "$IMAGE:$sha_tag"\n\n'
            "      - name: Check the image contract",
            1,
        )
        self.assertNotEqual(seeded, self.text)
        self.assertTrue(
            any("runs after the image is published" in problem for problem in problems(seeded))
        )


if __name__ == "__main__":
    unittest.main()
