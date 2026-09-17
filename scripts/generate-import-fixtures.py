#!/usr/bin/env python3
"""Original tiny CUR/ANI/ZIP fixtures; no third-party cursor artwork."""
import struct as s, zlib, zipfile
from pathlib import Path
root=Path(__file__).resolve().parent.parent/'tests/import-fixtures'
root.mkdir(exist_ok=True)
def cur(color, png=False, sizes=((3,2),(6,4))):
    entries=[]; bodies=[]; offset=6+16*len(sizes)
    for w,h in sizes:
        rgba=bytes(color)+bytes([0,0,0,0])*(w*h-1)
        if png:
            def chunk(tag,data):return s.pack('>I',len(data))+tag+data+s.pack('>I',zlib.crc32(tag+data))
            body=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',s.pack('>IIBBBBB',w,h,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b''.join(b'\0'+rgba[y*w*4:(y+1)*w*4] for y in range(h))))+chunk(b'IEND',b'')
        else:
            bgra=b''.join(bytes([p[2],p[1],p[0],p[3]]) for p in [rgba[i:i+4] for i in range(0,len(rgba),4)])
            body=s.pack('<IiiHHIIIIII',40,w,h*2,1,32,0,0,0,0,0,0)+b''.join(bgra[y*w*4:(y+1)*w*4] for y in reversed(range(h)))+bytes(((w+31)//32)*4*h)
        entries.append(s.pack('<BBBBHHII',w,h,0,0,1,1,len(body),offset)); bodies.append(body); offset+=len(body)
    return s.pack('<HHH',0,2,len(sizes))+b''.join(entries+bodies)
def chunk(tag,data):return tag+s.pack('<I',len(data))+data+bytes(len(data)%2)
a=cur([200,100,50,128]);b=cur([20,60,200,255])
(root/'normal.cur').write_bytes(a);(root/'text.cur').write_bytes(cur([200,100,50,128],True))
ani=b'ACON'+chunk(b'anih',s.pack('<9I',36,2,3,0,0,0,0,3,3))+chunk(b'rate',s.pack('<3I',1,3,6))+chunk(b'seq ',s.pack('<3I',1,0,1))+chunk(b'LIST',b'fram'+chunk(b'icon',a)+chunk(b'icon',b))
(root/'busy.ani').write_bytes(b'RIFF'+s.pack('<I',len(ani))+ani)
(root/'unsupported.cur').write_bytes(b'unsupported')
with zipfile.ZipFile(root/'theme.zip','w',zipfile.ZIP_DEFLATED) as z:
    for n in ['normal.cur','busy.ani','text.cur']:z.writestr('Sample/'+n,(root/n).read_bytes())
    z.writestr('Sample/install.inf','[Do not execute]\n');z.writestr('Sample/launch.sh','exit 99\n')
with zipfile.ZipFile(root/'roots.zip','w',zipfile.ZIP_DEFLATED) as z:
    z.writestr('First/normal.cur',a)
    z.writestr('Second/normal.cur',b)
with zipfile.ZipFile(root/'diagonals.zip','w',zipfile.ZIP_DEFLATED) as z:
    for name in ['Diagonal Resize 1.ani', 'Diagonal Resize 2.ani']:
        z.writestr('Sample/'+name,(root/'busy.ani').read_bytes())
print(root)

# Synthetic compatibility sample: inclusive RIFF size and explicit image hints.
compat_root=root.parent/'ani-compat'
compat_root.mkdir(exist_ok=True)
c1=cur([200,100,50,128],sizes=[(160,160)])
c2=cur([20,60,200,255],sizes=[(160,160)])
body=b'ACON'+chunk(b'anih',s.pack('<9I',36,2,3,160,160,32,1,3,3))+chunk(b'rate',s.pack('<3I',1,3,6))+chunk(b'seq ',s.pack('<3I',1,0,1))+chunk(b'LIST',b'fram'+chunk(b'icon',c1)+chunk(b'icon',c2))
(compat_root/'busy.ani').write_bytes(b'RIFF'+s.pack('<I',len(body)+8)+body)
