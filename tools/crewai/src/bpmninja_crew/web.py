"""Internetrecherche ohne bezahlte Such-API.

Primärquelle ist der RSS-Endpunkt von Bing, Fallback ist DuckDuckGo HTML.
Webseiten werden mit BeautifulSoup auf lesbaren Text reduziert. Private
und lokale Adressen sind gesperrt, damit die Crew keine internen Dienste
abfragt.
"""

from __future__ import annotations

import html
import ipaddress
import os
import re
import socket
import xml.etree.ElementTree as ET
from urllib.parse import urlparse

import requests
from bs4 import BeautifulSoup

USER_AGENT = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36"
)
TIMEOUT = 25
MAX_PAGE_CHARS = 60_000


def _clean(text: str) -> str:
    text = re.sub(r"<[^>]+>", " ", text or "")
    return re.sub(r"\s+", " ", html.unescape(text)).strip()


def parse_bing_rss(payload: bytes) -> list[dict[str, str]]:
    """Bing-RSS in Ergebnisliste umwandeln (separat testbar)."""
    root = ET.fromstring(payload)
    results: list[dict[str, str]] = []
    for item in root.findall(".//item"):
        results.append(
            {
                "title": _clean(item.findtext("title", default="")),
                "url": (item.findtext("link", default="") or "").strip(),
                "snippet": _clean(item.findtext("description", default="")),
                "published": (item.findtext("pubDate", default="") or "").strip(),
            }
        )
    return results


def _bing(query: str, max_results: int) -> list[dict[str, str]]:
    response = requests.get(
        "https://www.bing.com/search",
        params={"format": "rss", "q": query, "count": max_results},
        headers={"User-Agent": USER_AGENT, "Accept": "application/rss+xml,text/xml"},
        timeout=TIMEOUT,
    )
    response.raise_for_status()
    return parse_bing_rss(response.content)[:max_results]


def _duckduckgo(query: str, max_results: int) -> list[dict[str, str]]:
    response = requests.post(
        "https://html.duckduckgo.com/html/",
        data={"q": query},
        headers={"User-Agent": USER_AGENT},
        timeout=TIMEOUT,
    )
    response.raise_for_status()
    soup = BeautifulSoup(response.text, "html.parser")
    results: list[dict[str, str]] = []
    for node in soup.select(".result"):
        link = node.select_one("a.result__a")
        if not link:
            continue
        snippet = node.select_one(".result__snippet")
        results.append(
            {
                "title": link.get_text(" ", strip=True),
                "url": link.get("href", ""),
                "snippet": snippet.get_text(" ", strip=True) if snippet else "",
                "published": "",
            }
        )
        if len(results) >= max_results:
            break
    return results


def web_search(query: str, max_results: int = 6) -> list[dict[str, str]]:
    """Websuche mit Bing-RSS und DuckDuckGo-Fallback."""
    errors: list[str] = []
    for provider in (_bing, _duckduckgo):
        try:
            results = provider(query, max_results)
            if results:
                return results
        except Exception as exc:  # noqa: BLE001 - Suche darf den Lauf nicht abbrechen
            errors.append(f"{provider.__name__}: {exc}")
    raise RuntimeError("Websuche fehlgeschlagen: " + "; ".join(errors))


def _assert_public_url(url: str) -> None:
    parsed = urlparse(url)
    if parsed.scheme not in {"http", "https"}:
        raise ValueError("Nur http/https ist erlaubt.")
    if not parsed.hostname:
        raise ValueError("URL ohne Hostname.")
    try:
        address = ipaddress.ip_address(parsed.hostname)
    except ValueError:
        try:
            address = ipaddress.ip_address(socket.gethostbyname(parsed.hostname))
        except OSError as exc:
            raise ValueError(f"Hostname nicht auflösbar: {parsed.hostname}") from exc
    if (
        address.is_private
        or address.is_loopback
        or address.is_link_local
        or address.is_reserved
    ):
        raise ValueError("Lokale oder private Adressen sind gesperrt.")


def fetch_url(url: str, max_chars: int = 12_000) -> str:
    """Webseite laden und als lesbaren Text zurückgeben."""
    _assert_public_url(url)
    response = requests.get(
        url,
        headers={"User-Agent": USER_AGENT, "Accept": "text/html,text/plain,*/*"},
        timeout=TIMEOUT,
        allow_redirects=True,
    )
    response.raise_for_status()
    content_type = response.headers.get("content-type", "")
    if "html" not in content_type and not response.text.lstrip().startswith("<"):
        text = response.text
        return f"# {url}\n{text[:max_chars]}"
    soup = BeautifulSoup(response.text, "html.parser")
    for tag in soup(["script", "style", "nav", "footer", "header", "noscript", "svg"]):
        tag.decompose()
    title = soup.title.get_text(" ", strip=True) if soup.title else url
    main = soup.find("main") or soup.find("article") or soup.body or soup
    text = re.sub(r"\n{3,}", "\n\n", main.get_text("\n", strip=True))
    if len(text) > max_chars:
        text = text[:max_chars] + "\n… gekürzt"
    return f"# {title}\nQuelle: {response.url}\n\n{text}"


def github_search(query: str, kind: str = "repositories", max_results: int = 5) -> str:
    """GitHub-Suche für Repos, Issues und (mit Token) Code."""
    headers = {"Accept": "application/vnd.github+json", "User-Agent": USER_AGENT}
    token = os.getenv("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if kind not in {"repositories", "issues", "code"}:
        raise ValueError("kind muss repositories, issues oder code sein.")
    if kind == "code" and not token:
        return "Code-Suche benötigt GITHUB_TOKEN; nutze repositories oder issues."
    response = requests.get(
        f"https://api.github.com/search/{kind}",
        params={"q": query, "per_page": min(max_results, 10)},
        headers=headers,
        timeout=TIMEOUT,
    )
    if response.status_code == 403:
        return "GitHub-Rate-Limit erreicht; später erneut versuchen."
    response.raise_for_status()
    items = response.json().get("items", [])[:max_results]
    lines: list[str] = [f"# GitHub {kind}: {query}"]
    for item in items:
        if kind == "repositories":
            lines.append(
                f"- {item.get('full_name')} ★{item.get('stargazers_count', 0)} "
                f"({item.get('language') or 'n/a'}, Stand {item.get('updated_at', '')[:10]})\n"
                f"  {item.get('description') or ''}\n  {item.get('html_url')}"
            )
        else:
            lines.append(
                f"- {item.get('title')} [{item.get('state', '')}]\n"
                f"  {item.get('html_url')}\n  {(item.get('body') or '')[:400]}"
            )
    return "\n".join(lines)
