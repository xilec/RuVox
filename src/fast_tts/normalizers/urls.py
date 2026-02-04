"""URL and path normalizer."""

import re
from urllib.parse import urlparse

from .numbers import NumberNormalizer


class URLPathNormalizer:
    """Normalizes URLs, emails, IPs and file paths."""

    # Protocol pronunciations
    PROTOCOLS = {
        'https': 'эйч ти ти пи эс',
        'http': 'эйч ти ти пи',
        'ftp': 'эф ти пи',
        'ssh': 'эс эс эйч',
        'git': 'гит',
        'file': 'файл',
        'sftp': 'эс эф ти пи',
        'ws': 'веб сокет',
        'wss': 'веб сокет секьюр',
    }

    # Top-level domain pronunciations
    TLD_MAP = {
        'com': 'ком',
        'org': 'орг',
        'net': 'нет',
        'ru': 'ру',
        'io': 'ай оу',
        'dev': 'дев',
        'app': 'апп',
        'ai': 'эй ай',
        'co': 'ко',
        'me': 'ми',
        'uk': 'ю кей',
        'edu': 'еду',
        'gov': 'гов',
        'info': 'инфо',
        'biz': 'биз',
    }

    # Drive letter map (for Windows paths)
    DRIVE_LETTERS = {
        'c': 'си',
        'd': 'ди',
        'e': 'и',
        'f': 'эф',
        'g': 'джи',
        'h': 'эйч',
    }

    def __init__(self):
        self.number_normalizer = NumberNormalizer()

    def normalize_url(self, url: str) -> str:
        """Convert URL to speakable text."""
        if not url:
            return url

        parsed = urlparse(url)
        parts = []

        # Protocol
        scheme = parsed.scheme.lower()
        if scheme in self.PROTOCOLS:
            parts.append(self.PROTOCOLS[scheme])
        else:
            parts.append(scheme)

        parts.append('двоеточие слэш слэш')

        # Host (domain)
        host = parsed.netloc
        port = None

        # Extract port if present
        if ':' in host:
            host_parts = host.rsplit(':', 1)
            host = host_parts[0]
            port = host_parts[1]

        # Domain parts with TLD handling
        domain_parts = host.split('.')
        domain_words = []
        for i, part in enumerate(domain_parts):
            if i == len(domain_parts) - 1 and part.lower() in self.TLD_MAP:
                # Last part is TLD
                domain_words.append(self.TLD_MAP[part.lower()])
            elif part.replace('.', '').isdigit():
                # Numeric part like version number (3.11)
                domain_words.append(self.number_normalizer.normalize_number(part))
            else:
                domain_words.append(part)

        parts.append(' точка '.join(domain_words))

        # Port
        if port:
            parts.append('двоеточие')
            parts.append(self.number_normalizer.normalize_number(port))

        # Path
        path = parsed.path
        if path and path != '/':
            path_segments = path.strip('/').split('/')
            for segment in path_segments:
                parts.append('слэш')
                # Handle segments with dots (versions, extensions)
                if '.' in segment:
                    segment_parts = segment.split('.')
                    segment_words = []
                    for sp in segment_parts:
                        if sp.isdigit():
                            segment_words.append(self.number_normalizer.normalize_number(sp))
                        else:
                            segment_words.append(sp)
                    parts.append(' точка '.join(segment_words))
                else:
                    parts.append(segment)

        # Query params (simplified - just include key parts)
        if parsed.query:
            parts.append('вопросительный знак')
            query_parts = parsed.query.split('&')
            for qp in query_parts:
                if '=' in qp:
                    key, value = qp.split('=', 1)
                    parts.extend([key, 'равно', value])

        # Fragment
        if parsed.fragment:
            parts.append('решётка')
            parts.append(parsed.fragment)

        return ' '.join(parts)

    def normalize_email(self, email: str) -> str:
        """Convert email to speakable text."""
        if not email or '@' not in email:
            return email

        local_part, domain = email.rsplit('@', 1)
        parts = []

        # Local part (before @)
        local_normalized = self._normalize_identifier(local_part)
        parts.append(local_normalized)

        parts.append('собака')

        # Domain part
        domain_parts = domain.split('.')
        domain_words = []
        for i, part in enumerate(domain_parts):
            if i == len(domain_parts) - 1 and part.lower() in self.TLD_MAP:
                domain_words.append(self.TLD_MAP[part.lower()])
            else:
                domain_words.append(part)

        parts.append(' точка '.join(domain_words))

        return ' '.join(parts)

    def _normalize_identifier(self, identifier: str) -> str:
        """Normalize email local part or similar identifier."""
        result = []
        current_word = ''
        i = 0
        while i < len(identifier):
            char = identifier[i]
            if char == '.':
                if current_word:
                    result.append(current_word)
                    current_word = ''
                result.append('точка')
            elif char == '_':
                if current_word:
                    result.append(current_word)
                    current_word = ''
                result.append('андерскор')
            elif char == '-':
                if current_word:
                    result.append(current_word)
                    current_word = ''
                result.append('дефис')
            elif char.isdigit():
                if current_word:
                    result.append(current_word)
                    current_word = ''
                # Collect consecutive digits
                num_str = ''
                while i < len(identifier) and identifier[i].isdigit():
                    num_str += identifier[i]
                    i += 1
                result.append(self.number_normalizer.normalize_number(num_str))
                continue
            else:
                current_word += char
            i += 1

        if current_word:
            result.append(current_word)

        return ' '.join(result)

    def normalize_ip(self, ip: str) -> str:
        """Convert IP address to speakable text."""
        if not ip:
            return ip

        octets = ip.split('.')
        if len(octets) != 4:
            return ip

        parts = []
        for i, octet in enumerate(octets):
            try:
                num = int(octet)
                parts.append(self.number_normalizer.normalize_number(str(num)))
            except ValueError:
                parts.append(octet)

        return ' точка '.join(parts)

    def normalize_filepath(self, path: str) -> str:
        """Convert file path to speakable text."""
        if not path:
            return path

        parts = []

        # Detect path type and separator
        if '\\' in path:
            # Windows path
            segments = path.split('\\')
            separator = 'бэкслэш'
        else:
            # Unix path
            segments = path.split('/')
            separator = 'слэш'

        for i, segment in enumerate(segments):
            if i > 0:
                parts.append(separator)

            if not segment:
                # Empty segment (from leading slash or double slash)
                continue

            # Handle special segments
            if segment == '~':
                parts.append('тильда')
            elif segment == '.':
                parts.append('точка')
            elif segment == '..':
                parts.append('точка точка')
            elif len(segment) == 2 and segment[1] == ':' and segment[0].isalpha():
                # Windows drive letter (e.g., C:)
                drive = segment[0].lower()
                if drive in self.DRIVE_LETTERS:
                    parts.append(self.DRIVE_LETTERS[drive])
                else:
                    parts.append(drive)
                parts.append('двоеточие')
            elif segment.startswith('.'):
                # Hidden file/directory (starts with .)
                parts.append('точка')
                rest = segment[1:]
                if '.' in rest:
                    # Has extension
                    name_parts = rest.rsplit('.', 1)
                    parts.append(self._normalize_filename_part(name_parts[0]))
                    parts.append('точка')
                    parts.append(name_parts[1])
                else:
                    parts.append(rest)
            elif '.' in segment:
                # Filename with extension(s)
                # Handle multiple dots (e.g., test.spec.ts)
                dot_parts = segment.split('.')
                for j, dp in enumerate(dot_parts):
                    if j > 0:
                        parts.append('точка')
                    parts.append(self._normalize_filename_part(dp))
            else:
                # Regular segment
                parts.append(self._normalize_filename_part(segment))

        return ' '.join(parts)

    def _normalize_filename_part(self, part: str) -> str:
        """Normalize a filename part, handling dashes and other special chars."""
        if '-' in part:
            subparts = part.split('-')
            return ' дефис '.join(subparts)
        return part
