#!/usr/bin/env python3
"""Read generated CUR/ANI conversions with system libXcursor. No display/settings."""
import ctypes as c, os, subprocess, tempfile
from pathlib import Path
repo=Path(__file__).resolve().parent.parent
class Image(c.Structure):
    _fields_=[(k,c.c_uint32) for k in ['version','size','width','height','xhot','yhot','delay']]+[('pixels',c.POINTER(c.c_uint32))]
class Images(c.Structure):
    _fields_=[('nimage',c.c_int),('images',c.POINTER(c.POINTER(Image))),('name',c.c_char_p)]
lib=c.CDLL('libXcursor.so.1')
lib.XcursorFilenameLoadImages.argtypes=[c.c_char_p,c.c_int]
lib.XcursorFilenameLoadImages.restype=c.POINTER(Images)
lib.XcursorImagesDestroy.argtypes=[c.POINTER(Images)]
lib.XcursorLibraryLoadImages.argtypes=[c.c_char_p,c.c_char_p,c.c_int]
lib.XcursorLibraryLoadImages.restype=c.POINTER(Images)
with tempfile.TemporaryDirectory(prefix='cursormochi-reference-') as tmp:
    subprocess.run(['cargo','run','--locked','-q','-p','cursormochi-platform','--example','validate-import','--',tmp],cwd=repo,check=True)
    os.environ['XCURSOR_PATH']=tmp
    for role in ['left_ptr','text','watch','progress', 'e-resize','w-resize','n-resize','s-resize','col-resize','row-resize', 'nw-resize','se-resize','ne-resize','sw-resize', 'left_side','right_side','top_side','bottom_side', 'top_left_corner','bottom_right_corner','top_right_corner','bottom_left_corner']:
        for w,h in ([(n,n) for n in [16,24,32,48,64,96,128,160]] if role=='progress' else [(3,2),(6,4)]):
            for lookup in ['file', 'theme/name']:
                images = (lib.XcursorFilenameLoadImages(str(Path(tmp)/'Independent-Import/cursors'/role).encode(),w) if lookup == 'file' else lib.XcursorLibraryLoadImages(role.encode(), b'Independent-Import', w))
                assert images,(role,w)
                count=1 if role in ['left_ptr','text'] else 3
                assert images.contents.nimage==count
                for i in range(count):
                    f=images.contents.images[i].contents
                    hotspot = ((w + 80)//160) if role=='progress' else 1
                    assert (f.size,f.width,f.height,f.xhot,f.yhot)==(w,w,h,hotspot,hotspot)
                    assert f.delay==([17,50,100][i] if count==3 else 0)
                    expected=0xff143cc8 if count==3 and i!=1 else 0x80643219
                    if role=='progress' and w!=160:
                        expected=sum((((expected >> shift & 255)*w*w+12800)//25600) << shift for shift in [0,8,16,24])
                    assert list(f.pixels[:w*h])==[expected]+[0]*(w*h-1),(role,w,i)
                lib.XcursorImagesDestroy(images)
        print(role,': PASS independent libXcursor file + theme/name lookup, pixels, alpha, dimensions, hotspots, order and integer delays')
