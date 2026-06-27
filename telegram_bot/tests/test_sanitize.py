"""Tests for the input-sanitization and Markdown-safety helpers."""
from __future__ import annotations

from bot import sanitize as s


def test_sanitize_text_strips_control_and_zero_width_and_collapses():
    raw = "  Angry​\tBirds\x07  \n HD  "
    assert s.sanitize_text(raw, 100) == "Angry Birds HD"


def test_sanitize_text_caps_length():
    assert s.sanitize_text("x" * 500, 10) == "x" * 10


def test_sanitize_text_none_is_empty():
    assert s.sanitize_text(None, 10) == ""


def test_sanitize_multiline_keeps_paragraphs_but_trims():
    raw = "line one   \n\n\n\nline two‍\t end"
    assert s.sanitize_multiline(raw, 100) == "line one\n\nline two end"


def test_word_count():
    assert s.word_count("a b c") == 3
    assert s.word_count("one") == 1


def test_is_valid_version_accepts_numbers_and_optional_v():
    for good in ("1", "1.0", "v1.0", "V1.2.3", "12.34.56"):
        assert s.is_valid_version(good), good


def test_is_valid_version_rejects_junk():
    for bad in ("", "abc", "1.0a", "v", "1..0", "beta", "1.0 (build 3)"):
        assert not s.is_valid_version(bad), bad


def test_looks_binary_true_for_replacement_heavy_text():
    assert s.looks_binary("A" + "�" * 50 + "B")


def test_looks_binary_false_for_real_log():
    assert not s.looks_binary("touchHLE UNOFFICIAL abc123 — https://touchhle.org/\n" * 50)


def test_looks_binary_false_for_a_couple_of_replacements():
    # A handful of odd bytes in a long file shouldn't trip the heuristic.
    assert not s.looks_binary("normal log text " * 100 + "��")


def test_extract_links_strips_trailing_punctuation_and_dedupes():
    text = (
        "Try http://a/x.ipa. and again http://a/x.ipa, "
        "plus (http://b/y.ipa) and <http://c/z.ipa>"
    )
    assert s.extract_links(text) == [
        "http://a/x.ipa",
        "http://b/y.ipa",
        "http://c/z.ipa",
    ]


def test_extract_links_caps_count():
    text = " ".join(f"http://h/{i}.ipa" for i in range(50))
    assert len(s.extract_links(text, limit=5)) == 5


def test_gh_inline_code_neutralizes_link_injection():
    out = s.gh_inline_code("x](http://evil)")
    assert out == "`x](http://evil)`"


def test_gh_inline_code_escapes_backticks():
    assert "`" not in s.gh_inline_code("a`b`c")[1:-1]


def test_gh_inline_code_empty():
    assert s.gh_inline_code("") == "_n/a_"


def test_md_escape_escapes_specials():
    assert s.md_escape("a_b*c[d]") == "a\\_b\\*c\\[d]"


def test_safe_filename_blocks_traversal():
    assert s.safe_filename("../../etc/passwd") == "passwd"
    assert s.safe_filename("a/b/c.txt") == "c.txt"
    assert s.safe_filename(None, fallback="x.bin") == "x.bin"
    assert s.safe_filename("!!!", fallback="x.bin") == "x.bin"


def test_clamp_and_fits():
    assert s.clamp("abcdef", 3) == "abc"
    assert s.fits_telegram("x" * s.TELEGRAM_MAX_MESSAGE)
    assert not s.fits_telegram("x" * (s.TELEGRAM_MAX_MESSAGE + 1))
