"""Generate small MDX/MDD fixtures, without distributing third-party dictionaries."""
import io
import math
from pathlib import Path
import struct
import sys
import wave
import zlib


def container(path, records, title, mdd=False):
    records = sorted(records.items() if isinstance(records, dict) else records, key=lambda record: record[0])
    encoding = 'utf-16le' if mdd else 'utf-8'
    term = b'\0\0' if mdd else b'\0'
    header = (f'<Dictionary GeneratedByEngineVersion="2.0" Encrypted="0" Encoding="UTF-8" Title="{title}"/>').encode('utf-16le')
    pack = lambda *values: struct.pack('>' + 'Q' * len(values), *values)
    block = lambda data: b'\x02\0\0\0' + struct.pack('>I', zlib.adler32(data)) + zlib.compress(data)
    keys, data = b'', b''
    for key, value in records:
        keys += pack(len(data)) + key.encode(encoding) + term
        data += value.encode() + (b'' if mdd else b'\0') if isinstance(value, str) else value
    keys_block = block(keys)
    info = pack(len(records))
    for key in (records[0][0], records[-1][0]):
        encoded = key.encode(encoding)
        info += struct.pack('>H', len(encoded) // (2 if mdd else 1)) + encoded + term
    info += pack(len(keys_block), len(keys))
    info_block = block(info)
    key_header = pack(1, len(records), len(info), len(info_block), len(keys_block))
    record_block = block(data)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(struct.pack('>I', len(header)) + header + struct.pack('<I', zlib.adler32(header))
                     + key_header + struct.pack('>I', zlib.adler32(key_header)) + info_block + keys_block
                     + pack(1, len(records), 16, len(record_block), len(record_block), len(data)) + record_block)


def create(root):
    root = Path(root)
    hello = '''<link rel="stylesheet" href="entry-theme.css"><script src="entry.js"></script>
<xhtml:ol start="1"><xhtml:li><def>used as a <xr-gs firstinblock="n"><a href="entry://greeting">GREETING</a></xr-gs> when meeting somebody</def></xhtml:li></xhtml:ol>
<xhtml:ol start="2"><xhtml:li>attract attention</xhtml:li></xhtml:ol>
<xhtml:ol start="3"><xhtml:li>express surprise</xhtml:li></xhtml:ol>
<img id="test-image" src="picture.svg"><img id="root-image" src="/picture.svg"><a id="test-sound" href="sound://hello.wav">Play pronunciation</a>
<p id="script-status">Script not loaded</p>'''
    container(root / 'nested' / 'Example.mdx', [('hello', hello), ('hello again', '<p>Second headword</p>'), ('greeting', '<p>A friendly welcome.</p>'), ('plum', '<p>A purple fruit.</p>'), ('plum', '@@@LINK=plumb'), ('plumb', '<p>Measure depth.</p>')], '测试词典')
    audio = io.BytesIO()
    with wave.open(audio, 'wb') as wav:
        wav.setnchannels(1); wav.setsampwidth(2); wav.setframerate(8000)
        wav.writeframes(b''.join(struct.pack('<h', int(1000 * math.sin(i * 2 * math.pi * 440 / 8000))) for i in range(2400)))
    container(root / 'nested' / 'Example.mdd', {
        '\\entry-theme.css': 'xr-gs {display:table} xr-gs[firstinblock="n"] {display:inline} def {color:rgb(12, 34, 56)} xhtml\\:li {margin-bottom:7px}',
        '\\entry.js': 'window.__dictionaryLoaded=true;document.addEventListener("DOMContentLoaded",()=>{document.getElementById("script-status").textContent="Dictionary JavaScript loaded"});',
        '\\picture.svg': '<svg xmlns="http://www.w3.org/2000/svg" width="80" height="40"><rect width="80" height="40" fill="green"/></svg>',
        '\\hello.wav': audio.getvalue(),
    }, 'Resources', mdd=True)
    container(root / 'Other.mdx', {'hello': '<h2>Hello from another dictionary</h2>'}, 'Second Dictionary')
    return root


if __name__ == '__main__':
    create(sys.argv[1])
