#!/usr/bin/env python3
"""Read-only differential checks against installed libXcursor. No X server needed."""
import ctypes as c
import os
from pathlib import Path
lib=c.CDLL('libXcursor.so.1')
lib.XcursorLibraryPath.restype=c.c_char_p
class Image(c.Structure):
    _fields_=[(k,c.c_uint32) for k in ['version','size','width','height','xhot','yhot','delay']]+[('pixels',c.POINTER(c.c_uint32))]
class Images(c.Structure):
    _fields_=[('nimage',c.c_int),('images',c.POINTER(c.POINTER(Image))),('name',c.c_char_p)]
lib.XcursorLibraryLoadImages.argtypes=[c.c_char_p,c.c_char_p,c.c_int]
lib.XcursorLibraryLoadImages.restype=c.POINTER(Images)
lib.XcursorImagesDestroy.argtypes=[c.POINTER(Images)]
root=Path(__file__).resolve().parent.parent/'tests/fixtures'
# This script is a dedicated subprocess; environment is not mutated in Rust tests.
os.environ['XCURSOR_PATH']=str(root)
for theme,count in [('Mochi-Light',1),('Mochi-Motion',4),('Mochi-Inherited',4)]:
    result=lib.XcursorLibraryLoadImages(b'left_ptr',theme.encode(),24)
    assert result and result.contents.nimage==count,(theme,'count')
    first=result.contents.images[0].contents
    assert (first.size,first.width,first.height,first.xhot,first.yhot)==(24,18,24,1,2), (theme, first.size,first.width,first.height,first.xhot,first.yhot)
    expected=0xffeaeaf0 if count==1 else 0xff64aaff
    assert first.pixels[0]==expected
    if count==4:
        assert [result.contents.images[i].contents.delay for i in range(4)]==[80,160,240,0]
        assert result.contents.images[1].contents.pixels[0]==0x80604020
    lib.XcursorImagesDestroy(result)
    print(theme,': PASS libXcursor nominal size, frames, pixels, hotspot, delay, inheritance')
