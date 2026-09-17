#!/usr/bin/env python3
"""Original MIT-licensed Xcursor fixtures; only writes tests/fixtures/."""
from pathlib import Path
import struct
root = Path(__file__).resolve().parent.parent / 'tests/fixtures'
def cursor(frames):
    chunks=[]
    for size,w,h,x,y,delay,color in frames:
        pixels=[]
        for yy in range(h):
            for xx in range(w):
                pixels.append(color if xx <= yy//2+2 and xx < w-2 else 0)
        chunks.append(struct.pack('<9I',36,0xfffd0002,size,1,w,h,x,y,delay)+struct.pack('<'+'I'*len(pixels),*pixels))
    pos=16+12*len(chunks);toc=[]
    for frame,chunk in zip(frames,chunks):
        toc.append(struct.pack('<3I',0xfffd0002,frame[0],pos));pos+=len(chunk)
    return b'Xcur'+struct.pack('<3I',16,65536,len(chunks))+b''.join(toc)+b''.join(chunks)
for name in ['Mochi-Light','Mochi-Motion','Mochi-Inherited','Mochi-Broken']:
    (root/name/'cursors').mkdir(parents=True,exist_ok=True)
    (root/name/'index.theme').write_text('[Icon Theme]\nName='+name+'\n'+('Inherits=Mochi-Motion\n' if name=='Mochi-Inherited' else ''))
static=cursor([(24,18,24,1,2,100,0xffeaeaf0),(32,24,32,2,3,100,0xffeaeaf0)])
animated=cursor([(24,18,24,1,2,delay,color) for delay,color in [(80,0xff64aaff),(160,0x80604020),(240,0xffec879c),(0,0xff64aaff)]])
(root/'Mochi-Light/cursors/left_ptr').write_bytes(static)
(root/'Mochi-Motion/cursors/left_ptr').write_bytes(animated)
(root/'Mochi-Motion/cursors/watch').write_bytes(animated)
(root/'Mochi-Broken/cursors/left_ptr').write_bytes(b'Xcur\x01')
for name in ['Mochi-Light','Mochi-Motion']:
    p=root/name/'cursors/default'
    if not p.is_symlink():p.symlink_to('left_ptr')
(root/'LICENSE').write_text('All fixture images and generator are original CursorMochi project material, licensed under MIT.\n')

# Distinct, original trial-region assets. Do not change reference left_ptr files.
for role, color in [('text',0xfff08080),('pointer',0xff80dd80),('move',0xffffbb55),
                    ('ew-resize',0xffbb88ff),('ns-resize',0xff88dddd),('progress',0xffdddd66)]:
    (root/'Mochi-Light/cursors'/role).write_bytes(cursor([(24,18,24,1,2,100,color)]))
