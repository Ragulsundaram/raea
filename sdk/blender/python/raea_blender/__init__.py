# raea's bridge for Blender: lets TypeScript (raea/blender) script this Blender through the same Python API a script
# would use. As an add-on it serves the running Blender (writing ~/.raea/blender/live.json for connect()); headless,
# python/headless.py runs it as the whole of a background Blender's run (raea/blender's launch(), raea blender run).
bl_info = {
    "name": "raea bridge",
    "author": "raea",
    "version": (0, 1, 1),
    "blender": (4, 2, 0),
    "location": "Starts with Blender",
    "description": "Lets raea/blender scripts in TypeScript drive this Blender",
    "category": "Development",
}


def register():
    from . import server
    server.start_gui()


def unregister():
    from . import server
    server.stop_gui()
