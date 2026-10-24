from bpmninja_crew.web import parse_bing_rss

RSS = b"""<?xml version="1.0"?>
<rss><channel>
<item>
  <title>BPMN &amp; Engines</title>
  <link>https://example.org/bpmn</link>
  <description>Ein &lt;b&gt;Vergleich&lt;/b&gt;.</description>
  <pubDate>Fri, 24 Oct 2026 10:00:00 GMT</pubDate>
</item>
</channel></rss>
"""


def test_parse_bing_rss():
    results = parse_bing_rss(RSS)
    assert results == [
        {
            "title": "BPMN & Engines",
            "url": "https://example.org/bpmn",
            "snippet": "Ein Vergleich .",
            "published": "Fri, 24 Oct 2026 10:00:00 GMT",
        }
    ]
