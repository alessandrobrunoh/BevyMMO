# blender_export_bevymmo.py
# ---------------------------------------------------------------------------
# Blender addon: Export scene as a BevyMMO map (.glb)
#
# Usage:
#   1. Install as addon: Edit → Preferences → Add-ons → Install → select this file
#   2. Or run from Text Editor: open this file, click Run Script
#   3. In the 3D Viewport sidebar (N key) → "BevyMMO Export" tab
#   4. Set map metadata, mark props, click "Export Map"
#
# Convention:
#   - Objects with custom property "bevymmo_kind" are exported as props
#   - Object named "__bevymmo_map_meta__" carries map-level metadata
#   - Custom properties per prop:
#       bevymmo_kind        (string)  — required, e.g. "tree_oak"
#       bevymmo_id          (string)  — optional, auto-generated if empty
#       bevymmo_collision    (enum)    — "none", "cylinder", "box", "sphere"
#       bevymmo_radius       (float)   — for cylinder/sphere collision
#       bevymmo_height       (float)   — for cylinder collision
#       bevymmo_half_extents (float[3])— for box collision (comma-separated)
#       bevymmo_blocks_move  (bool)    — default False
#       bevymmo_tint         (float[3])— RGB color (comma-separated)
# ---------------------------------------------------------------------------

bl_info = {
    "name": "BevyMMO Export",
    "author": "BevyMMO",
    "version": (1, 0, 0),
    "blender": (4, 2, 0),
    "location": "View3D > Sidebar > BevyMMO Export",
    "description": "Export Blender scenes as BevyMMO map files (.glb)",
    "category": "Import-Export",
}

import bpy
import json
import os
from math import degrees
from bpy.props import *
from bpy.types import Panel, Operator


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def get_or_create_scene_meta():
    """Find or create the __bevymmo_map_meta__ object."""
    name = "__bevymmo_map_meta__"
    obj = bpy.data.objects.get(name)
    if obj is None:
        mesh = bpy.data.meshes.new(name + "_mesh")
        obj = bpy.data.objects.new(name, mesh)
        bpy.context.collection.objects.link(obj)
        # Hide it from viewport and renders
        obj.hide_set(True)
        obj.hide_render = True
    return obj


def get_prop_objects():
    """Return all objects that have the bevymmo_kind custom property."""
    result = []
    for obj in bpy.data.objects:
        if obj.name.startswith("__bevymmo_"):
            continue  # Skip meta/marker objects
        if obj.get("bevymmo_kind"):
            result.append(obj)
    return result


def build_prop_extras(obj):
    """Build the bevymmo extras dict for a prop object."""
    kind = obj.get("bevymmo_kind", "")
    prop_id = obj.get("bevymmo_id", "")

    extras = {
        "kind": kind,
        "id": prop_id,
        "blocks_movement": bool(obj.get("bevymmo_blocks_move", False)),
    }

    # Collision shape
    collision_type = obj.get("bevymmo_collision", "none")
    if collision_type and collision_type != "none":
        collision = {"type": collision_type}
        if collision_type == "cylinder":
            collision["radius"] = float(obj.get("bevymmo_radius", 0.5))
            collision["height"] = float(obj.get("bevymmo_height", 2.0))
        elif collision_type == "sphere":
            collision["radius"] = float(obj.get("bevymmo_radius", 0.5))
        elif collision_type == "box":
            he_str = str(obj.get("bevymmo_half_extents", "1,1,1"))
            he_vals = [float(x.strip()) for x in he_str.split(",")]
            collision["half_extents"] = he_vals[:3] if len(he_vals) >= 3 else [1.0, 1.0, 1.0]
        extras["collision"] = collision

    # Tint
    tint_str = obj.get("bevymmo_tint", "")
    if tint_str:
        try:
            tint_vals = [float(x.strip()) for x in tint_str.split(",")]
            if len(tint_vals) >= 3:
                extras["tint"] = tint_vals[:3]
        except (ValueError, AttributeError):
            pass

    return extras


def build_map_extras(meta_obj):
    """Build the bevymmo extras dict for the map meta node."""
    return {
        "_meta": True,
        "map_id": meta_obj.get("bevymmo_map_id", "untitled"),
        "display_name": meta_obj.get("bevymmo_display_name", "Untitled Map"),
        "bounds": {
            "min_x": float(meta_obj.get("bevymmo_min_x", -20)),
            "max_x": float(meta_obj.get("bevymmo_max_x", 20)),
            "min_z": float(meta_obj.get("bevymmo_min_z", -20)),
            "max_z": float(meta_obj.get("bevymmo_max_z", 20)),
        },
    }


def apply_extras_to_gltf_node(gltf_node, extras_dict):
    """Write extras JSON onto a glTF export node.

    This uses the glTF I/O 'extras' mechanism which Blender's built-in
    exporter already supports — we just need to set the right custom property.
    """
    # Store as JSON string on the object; Blender's glTF exporter will
    # serialize it into the node's extras field automatically.
    return json.dumps({"bevymmo": extras_dict}, indent=2)


# ---------------------------------------------------------------------------
# UI Panel
# ---------------------------------------------------------------------------

class BEVYMMO_PT_export_panel(Panel):
    bl_label = "BevyMMO Export"
    bl_idname = "BEVYMMO_PT_export_panel"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "BevyMMO"

    def draw(self, context):
        layout = self.layout
        scene = context.scene

        # --- Map Metadata ---
        box = layout.box()
        box.label(text="Map Metadata:", icon="WORLD")
        box.prop(scene.bevymmo_props, "map_id")
        box.prop(scene.bevymmo_props, "display_name")
        col = box.column(align=True)
        col.label(text="Bounds:")
        row = col.row(align=True)
        row.prop(scene.bevymmo_props, "bounds_min_x", text="Min X")
        row.prop(scene.bevymmo_props, "bounds_max_x", text="Max X")
        row = col.row(align=True)
        row.prop(scene.bevymmo_props, "bounds_min_z", text="Min Z")
        row.prop(scene.bevymmo_props, "bounds_max_z", text="Max Z")

        # --- Quick Actions ---
        box = layout.box()
        box.label(text="Quick Actions:", icon="PLUS")
        box.operator("bevymmo.mark_selected_as_prop", icon="MESH_CIRCLE")
        box.operator("bevymmo.clear_bevymmo_props", icon="TRASH")
        box.label(text=f"Props in scene: {len(get_prop_objects())}")

        # --- Export ---
        box = layout.box()
        box.label(text="Export:", icon="EXPORT")
        box.operator("bevymmo.export_map_glb", icon="FILE_TICK")


# ---------------------------------------------------------------------------
# Property Group (stored on Scene)
# ---------------------------------------------------------------------------

class BevyMMO_Properties(PropertyGroup):
    map_id: StringProperty(
        name="Map ID",
        description="Stable identifier used by the game (e.g. 'starting_village')",
        default="untitled",
    )
    display_name: StringProperty(
        name="Display Name",
        description="Human-readable name shown in menus",
        default="Untitled Map",
    )
    bounds_min_x: FloatProperty(
        name="Min X", default=-20.0, precision=1,
    )
    bounds_max_x: FloatProperty(
        name="Max X", default=20.0, precision=1,
    )
    bounds_min_z: FloatProperty(
        name="Min Z", default=-20.0, precision=1,
    )
    bounds_max_z: FloatProperty(
        name="Max Z", default=20.0, precision=1,
    )


# ---------------------------------------------------------------------------
# Operators
# ---------------------------------------------------------------------------

class BEVYMMO_OT_mark_selected_as_prop(Operator):
    """Mark selected objects as BevyMMO props (adds required custom properties)."""
    bl_idname = "bevymmo.mark_selected_as_prop"
    bl_label = "Mark Selected as Prop"
    bl_options = {"REGISTER", "UNDO"}

    kind: StringProperty(
        name="Kind ID",
        description="Placeable kind (e.g. tree_oak, rock_01, house_simple)",
        default="cube",
    )

    def execute(self, context):
        selected = context.selected_objects
        if not selected:
            self.report({"WARNING"}, "No objects selected")
            return {"CANCELLED"}

        count = 0
        for obj in selected:
            if obj.name.startswith("__bevymmo_"):
                continue
            obj["bevymmo_kind"] = self.kind
            if not obj.get("bevymmo_id"):
                obj["bevymmo_id"] = f"prop_{count + 1:03d}"
            count += 1

        self.report({"INFO"}, f"Marked {count} objects as '{self.kind}' props")
        return {"FINISHED"}

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)


class BEVYMMO_OT_clear_bevymmo_props(Operator):
    """Remove all BevyMMO custom properties from all objects."""
    bl_idname = "bevymmo.clear_bevymmo_props"
    bl_label = "Clear All BevyMMO Props"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        keys_to_remove = [
            "bevymmo_kind", "bevymmo_id", "bevymmo_collision",
            "bevymmo_radius", "bevymmo_height", "bevymmo_half_extents",
            "bevymmo_blocks_move", "bevymmo_tint",
        ]
        count = 0
        for obj in bpy.data.objects:
            for key in keys_to_remove:
                if key in obj:
                    del obj[key]
                    count += 1

        self.report({"INFO"}, f"Cleared {count} properties")
        return {"FINISHED"}


class BEVYMMO_OT_export_map_glb(Operator):
    """Export the current scene as a BevyMMO .glb map file."""
    bl_idname = "bevymmo.export_map_glb"
    bl_label = "Export Map (.glb)"
    bl_options = {"REGISTER"}

    filepath: StringProperty(subtype="FILE_PATH")

    def execute(self, context):
        scene = context.scene
        props = scene.bevymmo_props

        # --- Build / update the meta object ---
        meta_obj = get_or_create_scene_meta()
        meta_obj["bevymmo_map_id"] = props.map_id
        meta_obj["bevymmo_display_name"] = props.display_name
        meta_obj["bevymmo_min_x"] = props.bounds_min_x
        meta_obj["bevymmo_max_x"] = props.bounds_max_x
        meta_obj["bevymmo_min_z"] = props.bounds_min_z
        meta_obj["bevymmo_max_z"] = props.bounds_max_z

        # --- Apply extras to all prop objects via custom property ---
        # We use a special property that the post-export hook reads
        prop_objs = get_prop_objects()
        for obj in prop_objs:
            extras = build_prop_extras(obj)
            obj["_bevymmo_extras_json_"] = json.dumps(extras)

        # Apply to meta object too
        map_extras = build_map_extras(meta_obj)
        meta_obj["_bevymmo_extras_json_"] = json.dumps(map_extras)

        # --- Export GLB ---
        filepath = self.filepath
        if not filepath.endswith(".glb"):
            filepath += ".glb"

        # Use Blender's built-in glTF exporter
        try:
            bpy.ops.export_scene.gltf(
                filepath=filepath,
                use_selection=False,
                export_format="GLB",
                export_apply=True,
                export_extras=True,  # This exports custom properties as glTF extras!
            )
        except RuntimeError as e:
            self.report({"ERROR"}, f"Export failed: {e}")
            return {"CANCELLED"}

        # Clean up temporary properties
        for obj in list(bpy.data.objects):
            if "_bevymmo_extras_json_" in obj:
                del obj["_bevymmo_extras_json_"]

        self.report({"INFO"}, f"Exported {len(prop_objs)} props to {filepath}")
        return {"FINISHED"}

    def invoke(self, context, event):
        context.window_manager.fileselect_add(self)
        return {"RUNNING_MODAL"}


# ---------------------------------------------------------------------------
# Registration
# ---------------------------------------------------------------------------

classes = (
    BevyMMO_Properties,
    BEVYMMO_PT_export_panel,
    BEVYMMO_OT_mark_selected_as_prop,
    BEVYMMO_OT_clear_bevymmo_props,
    BEVYMMO_OT_export_map_glb,
)


def register():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.Scene.bevymmo_props = PointerProperty(type=BevyMMO_Properties)


def unregister():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
    del bpy.types.Scene.bevymmo_props


if __name__ == "__main__":
    register()
