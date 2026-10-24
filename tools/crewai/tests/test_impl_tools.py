from pathlib import Path

import pytest

from bpmninja_crew.tools import impl_tools


@pytest.fixture()
def sample_repo(tmp_path: Path) -> Path:
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "lib.rs").write_text("fn a() {}\n", encoding="utf-8")
    (tmp_path / "HANDOFF.md").write_text("privat\n", encoding="utf-8")
    return tmp_path


def test_replace_requires_unique_match(sample_repo: Path):
    target = sample_repo / "src" / "lib.rs"
    target.write_text("fn a() {}\nfn a() {}\n", encoding="utf-8")
    with pytest.raises(ValueError):
        impl_tools.replace_text("src/lib.rs", "fn a() {}", "fn b() {}", root=sample_repo)


def test_replace_all(sample_repo: Path):
    target = sample_repo / "src" / "lib.rs"
    target.write_text("fn a() {}\nfn a() {}\n", encoding="utf-8")
    impl_tools.replace_text(
        "src/lib.rs", "fn a() {}", "fn b() {}", replace_all=True, root=sample_repo
    )
    assert target.read_text() == "fn b() {}\nfn b() {}\n"


def test_handoff_is_blocked(sample_repo: Path):
    with pytest.raises(ValueError):
        impl_tools.replace_text(
            "HANDOFF.md", "privat", "öffentlich", root=sample_repo
        )


def test_shell_metacharacters_are_rejected(sample_repo: Path):
    with pytest.raises(ValueError):
        impl_tools.run_command("cargo test; rm -rf /", root=sample_repo)


def test_unlisted_command_is_rejected(sample_repo: Path):
    with pytest.raises(ValueError):
        impl_tools.run_command("curl https://example.org", root=sample_repo)
