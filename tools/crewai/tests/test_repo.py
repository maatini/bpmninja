from pathlib import Path

import pytest

from bpmninja_crew import repo


@pytest.fixture()
def sample_repo(tmp_path: Path) -> Path:
    (tmp_path / "src").mkdir()
    (tmp_path / "src" / "lib.rs").write_text(
        "fn main() {\n    println!(\"hi\");\n}\n", encoding="utf-8"
    )
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "readme.md").write_text("# Titel\n", encoding="utf-8")
    (tmp_path / "target").mkdir()
    (tmp_path / "target" / "ignored.rs").write_text("fn x() {}\n", encoding="utf-8")
    return tmp_path


def test_list_skips_target(sample_repo: Path):
    result = repo.list_files("**/*", root=sample_repo)
    assert "src/lib.rs" in result
    assert "target/ignored.rs" not in result


def test_read_with_line_numbers(sample_repo: Path):
    result = repo.read_file("src/lib.rs", root=sample_repo)
    assert "1→fn main()" in result


def test_search_finds_pattern(sample_repo: Path):
    result = repo.search("println!", root=sample_repo)
    assert "src/lib.rs:2:" in result


def test_path_traversal_is_rejected(sample_repo: Path):
    with pytest.raises(ValueError):
        repo.read_file("../outside.rs", root=sample_repo)


def test_repo_stats(sample_repo: Path):
    result = repo.repo_stats(root=sample_repo)
    assert "Rust-Dateien: 1" in result
