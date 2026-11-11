# The Blender Python API as data, for tools/gen.ts: every RNA struct (bpy.types) with its properties and functions,
# every operator (bpy.ops) with its properties, and the modules written in C or Python outside RNA (mathutils, bmesh,
# bpy.utils, bpy.path, bpy.app, bpy.props, bpy_extras, ...) with their docstrings, whose `:type:` lines gen.ts reads.
# Python because it runs inside Blender:
#   blender --background --factory-startup --python tools/dump.py -- OUT.json
import bpy
import inspect
import json
import sys
import types

import _rna_info as rna_info

sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from refine import refine  # noqa: E402 (beside this file)


def prop(p):
    """One RNA property (a struct's, a function's parameter or an operator's) as a plain dict."""
    d = {"id": p.identifier, "type": p.type}
    if p.name and p.name != p.identifier:
        d["name"] = p.name
    if p.description:
        d["desc"] = p.description
    if p.fixed_type:
        d["fixed"] = p.fixed_type.identifier
    if p.type == "collection" and p.srna:
        d["srna"] = p.srna.identifier
    if p.array_length:
        d["len"] = p.array_length
        dims = [x for x in p.array_dimensions if x]
        if len(dims) > 1:
            d["dims"] = dims
    elif getattr(p.bl_prop, "is_array", False):
        d["dyn"] = True  # an array whose length depends on the data
    if p.subtype and p.subtype != "NONE":
        d["subtype"] = p.subtype
    if p.is_readonly:
        d["ro"] = True
    if p.is_never_none:
        d["never_none"] = True
    if p.is_required:
        d["required"] = True
    if p.is_argument_optional:
        d["optional"] = True
    if p.type == "enum":
        d["items"] = [[i, n, de] for (i, n, de, *_rest) in p.enum_items]
        if p.is_enum_flag:
            d["flag"] = True
        # an enum whose items come from the data (view_transform from the OCIO config, render.engine from the engines
        # registered) shows one placeholder item here, or none: its values are any string the data has
        if len(p.enum_items) <= 1:
            d["dynamic"] = True
    if p.type in {"int", "float"}:
        for k in ("min", "max"):
            v = getattr(p, k, None)
            if isinstance(v, (int, float)) and abs(v) < 1e30:
                d[k] = v
    if p.type == "string":
        st = getattr(p.bl_prop, "subtype", "")
        if st in {"FILE_PATH", "DIR_PATH", "FILE_NAME", "BYTE_STRING"}:
            d["subtype"] = st
    if p.default_str not in ("", "<UNKNOWN>", "None"):
        d["default"] = p.default_str if isinstance(p.default_str, str) else list(p.default_str)
    if p.deprecated:
        d["deprecated"] = [str(x) for x in p.deprecated]
    return d


def func(f):
    return {
        "id": f.identifier,
        "desc": f.description,
        "cls": f.is_classmethod,
        "args": [prop(a) for a in f.args],
        "ret": [prop(r) for r in f.return_values],
    }


def sig(obj):
    try:
        return str(inspect.signature(obj))
    except (TypeError, ValueError):
        return None


def doc(obj):
    d = getattr(obj, "__doc__", None)
    return inspect.cleandoc(d) if isinstance(d, str) else ""


def pyfunc(name, f):
    fn = getattr(f, "__func__", f)
    return {"id": name, "sig": sig(fn), "doc": doc(fn), "static": isinstance(f, (staticmethod, classmethod))}


def struct(s):
    d = {
        "id": s.identifier,
        "base": s.base.identifier if s.base else None,
        "name": s.name,
        "desc": s.description,
        "props": [prop(p) for p in s.properties],
        "funcs": [func(f) for f in s.functions],
    }
    if s.nested:
        d["nested"] = s.nested.identifier
    py = s.py_class
    # what Python adds on top of RNA (_bpy_types.py: Object.children, Mesh.from_pydata, ...)
    pyprops = []
    for name, attr in s.get_py_properties():
        if name in s.py_class.__dict__:
            pyprops.append({"id": name, "doc": doc(attr), "ro": attr.fset is None})
    pyfuncs = [pyfunc(n, py.__dict__.get(n, a)) for n, a in s.get_py_functions() if n in py.__dict__]
    cfuncs = [{"id": n, "doc": doc(a)} for n, a in s.get_py_c_functions() if n in py.__dict__]
    if pyprops:
        d["pyprops"] = pyprops
    if pyfuncs:
        d["pyfuncs"] = pyfuncs
    if cfuncs:
        d["cfuncs"] = cfuncs
    return d


def member(name, obj, depth):
    """A module or class member: its kind and docstring, a class's own members, a module's members (depth-limited)."""
    if isinstance(obj, types.ModuleType):
        return {"id": name, "kind": "module", "doc": doc(obj), "members": members(obj, depth + 1) if depth < 3 else []}
    if inspect.isclass(obj):
        own = []
        for n, a in obj.__dict__.items():
            if n.startswith("_") and n not in ("__init__", "__call__", "__len__", "__getitem__", "__iter__"):
                continue
            kind = "property" if isinstance(a, (types.GetSetDescriptorType, types.MemberDescriptorType, property)) else (
                "method" if callable(a) or isinstance(a, (staticmethod, classmethod)) else "data")
            m = {"id": n, "kind": kind, "doc": doc(a)}
            if kind == "method":
                m["sig"] = sig(getattr(a, "__func__", a))
                if isinstance(a, (staticmethod, classmethod)) or type(a).__name__ == "classmethod_descriptor":
                    m["static"] = True
            if kind == "data":
                m["type"] = type(a).__name__
                m["value"] = repr(a)[:200]
            if kind == "property" and isinstance(a, property):
                m["ro"] = a.fset is None
            own.append(m)
        bases = [b.__module__ + "." + b.__qualname__ for b in obj.__bases__ if b is not object]
        return {"id": name, "kind": "class", "doc": doc(obj), "sig": sig(obj), "bases": bases, "members": own}
    if callable(obj):
        return {"id": name, "kind": "function", "doc": doc(obj), "sig": sig(obj)}
    return {"id": name, "kind": "data", "type": type(obj).__name__, "value": repr(obj)[:200], "doc": ""}


def members(mod, depth=0):
    out = []
    for name in sorted(dir(mod)):
        if name.startswith("_"):
            continue
        try:
            obj = getattr(mod, name)
        except Exception:
            continue
        if isinstance(obj, types.ModuleType) and not obj.__name__.startswith(getattr(mod, "__name__", "")):
            continue  # an import, not a submodule
        out.append(member(name, obj, depth))
    return out


def app():
    """bpy.app is a struct sequence: its fields' values and docs, then its submodules."""
    out = []
    cls = type(bpy.app)
    for name in sorted(dir(bpy.app)):
        if name.startswith("_") or name in ("count", "index"):
            continue
        v = getattr(bpy.app, name)
        if isinstance(v, types.ModuleType) or type(v).__name__ in ("bpy_app_handlers", "module"):
            out.append(member(name, v, 1))
            continue
        d = doc(getattr(cls, name, None)) if hasattr(cls, name) else ""
        if callable(v):
            out.append({"id": name, "kind": "function", "doc": doc(v), "sig": sig(v)})
        else:
            out.append({"id": name, "kind": "data", "type": type(v).__name__, "value": repr(v)[:200], "doc": d})
    return out


MODULES = [
    "mathutils", "mathutils.geometry", "mathutils.noise", "mathutils.bvhtree", "mathutils.kdtree",
    "mathutils.interpolate", "bmesh", "bmesh.types", "bmesh.ops", "bmesh.utils", "bmesh.geometry",
    "bpy.utils", "bpy.utils.units", "bpy.utils.previews", "bpy.path", "bpy.props", "bpy.msgbus",
    "bpy_extras", "bpy_extras.object_utils", "bpy_extras.view3d_utils", "bpy_extras.io_utils",
    "bpy_extras.image_utils", "bpy_extras.mesh_utils", "bpy_extras.anim_utils", "bpy_extras.node_utils",
    "bl_math", "idprop.types", "imbuf", "imbuf.types", "gpu", "gpu.types", "gpu.matrix", "gpu.state", "gpu.shader",
    "gpu.texture", "gpu.platform", "gpu.capabilities", "gpu.select", "gpu.compute", "gpu_extras.batch",
    "gpu_extras.presets", "aud",
]


def module(name):
    """A module by import name, or an attribute of its parent (bpy.msgbus is one, not importable)."""
    import importlib
    try:
        return importlib.import_module(name)
    except ImportError:
        head, _, tail = name.rpartition(".")
        if not head:
            raise
        return getattr(module(head), tail)


def main():
    out = sys.argv[sys.argv.index("--") + 1]
    structs, _funcs, ops, _props = rna_info.BuildRNAInfo()
    data = {
        "blender": ".".join(str(x) for x in bpy.app.version),
        "version_string": bpy.app.version_string,
        "structs": [],
        "ops": [],
        "modules": {},
        "context": sorted(n for n in dir(bpy.context) if not n.startswith("_")),
        "bpy_struct": member("bpy_struct", bpy.types.bpy_struct, 1),
        "bpy_prop_collection": member("bpy_prop_collection", bpy.types.bpy_prop_collection, 1),
        "bpy_prop_array": member("bpy_prop_array", type(bpy.types.Object.bl_rna.properties["location"].default_array), 1),
        "bpy_func": member("bpy_func", bpy.types.bpy_func, 1) if hasattr(bpy.types, "bpy_func") else None,
        "app": app(),
    }
    for s in structs.values():
        s.build()
    for (_mod, ident), s in sorted(structs.items(), key=lambda kv: kv[0][1]):
        if "_OT_" in ident or s.is_operator_properties():
            continue
        data["structs"].append(struct(s))
    for op in sorted(ops.values(), key=lambda o: (o.module_name, o.func_name)):
        data["ops"].append({"mod": op.module_name, "name": op.func_name, "desc": op.description,
                            "args": [prop(a) for a in op.args]})
    for name in MODULES:
        try:
            mod = module(name)
        except Exception as e:
            print(f"dump: {name} not importable here: {e}", file=sys.stderr)
            continue
        data["modules"][name] = {"doc": doc(mod), "members": members(mod, 1)}
    data["refine"] = refine()
    with open(out, "w") as f:
        json.dump(data, f, separators=(",", ":"))
    print(f"dump: {len(data['structs'])} structs, {len(data['ops'])} operators, {len(data['modules'])} modules -> {out}")


main()
