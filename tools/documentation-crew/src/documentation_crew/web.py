"""Websuche und URL-Abruf ohne bezahlte Such-API.

Primärquelle ist der RSS-Endpunkt von Bing, Fallback ist DuckDuckGo HTML.
Private und lokale Adressen sind gesperrt, Redirects werden einzeln geprüft.
Damit eignet sich der Abruf auch für Läufe, in denen das Modell eine URL aus
einem Suchtreffer ungeprüft weiterreicht.
"""

from __future__ import annotations

import html
import ipaddress
import re
import socket
from urllib.parse import urljoin, urlparse

import requests
from bs4 import BeautifulSoup


USER_AGENT = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36"
)
TIMEOUT = 25
MAX_PAGE_BYTES = 2_097_152
MAX_REDIRECTS = 5


class WebError(RuntimeError):
    """Suche oder Abruf ist fehlgeschlagen."""


def _clean(text: str) -> str:
    text = re.sub(r"<[^>]+>", " ", text or "")
    return re.sub(r"\s+", " ", html.unescape(text)).strip()


def parse_bing_rss(payload: bytes) -> list[dict[str, str]]:
    """Bing-RSS in eine Ergebnisliste umwandeln (separat testbar)."""
    import xml.etree.ElementTree as ET

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
    query = query.strip()
    if len(query) < 3:
        raise WebError("Suchanfrage muss mindestens drei Zeichen haben.")
    errors: list[str] = []
    for provider in (_bing, _duckduckgo):
        try:
            results = provider(query, max_results)
        except Exception as exc:  # noqa: BLE001 - Suche darf den Lauf nicht abbrechen
            errors.append(f"{provider.__name__}: {exc}")
            continue
        if results:
            return results
    raise WebError("Websuche fehlgeschlagen: " + "; ".join(errors))


def _assert_public_url(url: str) -> None:
    parsed = urlparse(url)
    if parsed.scheme not in {"http", "https"}:
        raise WebError("Nur http/https ist erlaubt.")
    if not parsed.hostname:
        raise WebError("URL ohne Hostname.")
    try:
        address = ipaddress.ip_address(parsed.hostname)
    except ValueError:
        try:
            address = ipaddress.ip_address(socket.gethostbyname(parsed.hostname))
        except OSError as exc:
            raise WebError(f"Hostname nicht auflösbar: {parsed.hostname}") from exc
    if (
        address.is_private
        or address.is_loopback
        or address.is_link_local
        or address.is_reserved
        or address.is_multicast
        or address.is_unspecified
    ):
        raise WebError("Lokale oder private Adressen sind gesperrt.")


def _fetch_once(url: str) -> requests.Response:
    _assert_public_url(url)
    response = requests.get(
        url,
        headers={"User-Agent": USER_AGENT, "Accept": "text/html,text/plain,*/*"},
        timeout=TIMEOUT,
        allow_redirects=False,
        stream=True,
    )
    return response


def fetch_url(url: str, max_chars: int = 12_000) -> str:
    """Webseite laden, Redirects prüfen und als lesbaren Text zurückgeben."""
    current = url
    response: requests.Response | None = None
    for _ in range(MAX_REDIRECTS + 1):
        response = _fetch_once(current)
        if response.is_redirect:
            location = response.headers.get("location")
            response.close()
            if not location:
                raise WebError("Redirect ohne Location-Header.")
            current = urljoin(current, location)
            continue
        response.raise_for_status()
        break
    else:
        raise WebError("Zu viele Redirects.")
    if response is None:
        raise WebError("Kein Abruf erfolgt.")

    try:
        chunks: list[bytes] = []
        size = 0
        for chunk in response.iter_content(chunk_size=65_536):
            if not chunk:
                continue
            size += len(chunk)
            if size > MAX_PAGE_BYTES:
                chunks.append(chunk[: max(0, MAX_PAGE_BYTES - (size - len(chunk)))])
                break
            chunks.append(chunk)
        payload = b"".join(chunks)
    finally:
        response.close()

    content_type = response.headers.get("content-type", "")
    encoding = response.encoding or "utf-8"
    text = payload.decode(encoding, errors="replace")
    if "html" not in content_type and not text.lstrip().startswith("<"):
        return f"# {current}\n{text[:max_chars]}"

    soup = BeautifulSoup(text, "html.parser")
    for tag in soup(["script", "style", "nav", "footer", "header", "noscript", "svg"]):
        tag.decompose()
    title = soup.title.get_text(" ", strip=True) if soup.title else current
    main = soup.find("main") or soup.find("article") or soup.body or soup
    body = re.sub(r"\n{3,}", "\n\n", main.get_text("\n", strip=True))
    if len(body) > max_chars:
        body = body[:max_chars] + "\n… gekürzt"
    return f"# {title}\nQuelle: {current}\n\n{body}"
