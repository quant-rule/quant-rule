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
CHECK_STEPS = (NO_PYTHON, CONTRACT)


def image_job(text: str) -> str:
    start = text.find("\n  image:\n")
    end = text.find("\n  desktop:\n", start + 1)
    if start == -1 or end == -1:
        return ""
    return text[start:end]


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

    job = image_job(text)
    if not job:
        return found + ["the image job is missing"]
    order = steps(job)
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
            '              .\n          else\n            docker build --platform linux/amd64 --tag "$REF" .',
            '              .\n            docker push "$REF"\n          else\n            docker build --platform linux/amd64 --tag "$REF" .',
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
