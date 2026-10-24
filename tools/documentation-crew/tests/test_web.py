from __future__ import annotations

import pytest

from documentation_crew import web


def test_parse_bing_rss():
    payload = b"""<?xml version="1.0"?>
    <rss><channel><item>
      <title>Diataxis</title>
      <link>https://diataxis.fr/</link>
      <description>Ein Framework.</description>
      <pubDate>Fri, 24 Oct 2026 10:00:00 GMT</pubDate>
    </item></channel></rss>"""
    results = web.parse_bing_rss(payload)
    assert results == [
        {
            "title": "Diataxis",
            "url": "https://diataxis.fr/",
            "snippet": "Ein Framework.",
            "published": "Fri, 24 Oct 2026 10:00:00 GMT",
        }
    ]


@pytest.mark.parametrize(
    "url",
    [
        "http://127.0.0.1:8080/",
        "http://localhost/",
        "http://[::1]/",
        "file:///etc/passwd",
        "ftp://example.com/file",
    ],
)
def test_private_and_invalid_urls_are_rejected(url: str):
    with pytest.raises(web.WebError):
        web._assert_public_url(url)


def test_empty_search_is_rejected():
    with pytest.raises(web.WebError):
        web.web_search("x")
