import math
import time

from pie.audio import AudioSource  # pyright: ignore[reportMissingModuleSource]
from pie.engine import Engine  # pyright: ignore[reportMissingModuleSource]
from pie.entity import Camera, Entity  # pyright: ignore[reportMissingModuleSource]
from pie.light import Light  # pyright: ignore[reportMissingModuleSource]
from pie.mesh import Mesh  # pyright: ignore[reportMissingModuleSource]
from pie.scene import Scene
from pie.texture import Texture  # pyright: ignore[reportMissingModuleSource]

engine = Engine("love me some pie engine", 800, 600)

engine.configure_point_lights(8)

camera = Camera(0.0, 1.0, 5.0)
engine.camera = camera

camera.fov = 90.0

scene = Scene()

e1 = Entity()
mesh = Mesh.from_obj("n64_logo.obj")
mesh.set_texture(Texture.from_color(255, 0, 0, 255))
e1.set_mesh(mesh)
e1.rotation = (0.0, math.radians(45.0), 0.0)
engine.add_entity(e1)

e2 = Entity()
e2.position = (2.0, 0.0, -1.0)
e2.set_mesh(Mesh.from_obj("teapot.obj"))
e2.scale = (0.5, 0.5, 0.5)
engine.add_entity(e2)

# Ground
g = Entity()
g.scale = (100.0, 1.0, 100.0)
g.set_mesh(Mesh.plane())
engine.add_entity(g)

directional_light = Light.directional(0.0, -1.0, 0.0)  # direction, color
engine.add_light(directional_light)

red_light = Light.point(1.0, 0.0, 0.0, 15.0)  # position, color, radius
# engine.add_light(red_light)

blue_light = Light.point(0.0, 0.3, 1.0, 15.0)
engine.add_light(blue_light)

audio = AudioSource.from_wav("test.wav", True)
# audio2 = AudioSource.new_sine(220.0, True)

engine.add_audio_source(audio)
# engine.add_audio_source(audio2)
audio.play()
# audio2.play()

running = True
t = 0.0
yaw = -90.0

dt = 1 / 120

is_captured = False

while running:
    # Poll events from the engine and handle Quit/Window/KeyDown events.
    # This ensures window close / quit requests and key presses are processed
    # even if the rest of the loop relies on key polling.
    for event in engine.poll_events():
        en = getattr(event, "name", "")
        if en == "Quit":
            running = False
            break
        if en == "Window":
            ev = event.fields.get("event", "") if getattr(event, "fields", None) else ""
            if str(ev).lower() in ("close", "closerequested", "close_requested"):
                running = False
                break
        if en == "KeyDown":
            key = (
                event.fields.get("key")
                or event.fields.get("keycode")
                or event.fields.get("scancode")
                or event.fields.get("sym")
                or event.fields.get("text")
                or ""
            )
            key = str(key)
            if key in ("Escape", "Esc"):
                running = False
                break
            if key == "Tab":
                is_captured = not is_captured
                engine.set_mouse_capture(is_captured)
    if not running:
        break

    t += dt

    angle = t * 2.0
    radius = 10.0
    y = 4.0

    # red rotates with +angle
    x = math.cos(angle) * radius
    z = math.sin(angle) * radius
    red_light.position = (x, y, z)
    audio.position = (x, y, z)

    # blue rotates with -angle
    angle_b = -angle
    xb = math.cos(angle_b) * radius
    zb = math.sin(angle_b) * radius
    blue_light.position = (xb, y, zb)
    # audio2.position = (xb, y, zb)

    # Tab toggling is handled from the engine.poll_events() loop at the top of the frame
    # to avoid rapid toggles from key-repeat and to centralize input handling.

    rel = engine.get_mouse_rel()
    if rel[0] != 0 or rel[1] != 0:
        sens = 0.05
        engine.rotate_camera(rel[0] * sens, -rel[1] * sens)
        yaw += rel[0] * sens

    rad = math.radians(yaw)
    fx = math.cos(rad)
    fz = math.sin(rad)
    rx = -fz
    rz = fx

    speed = 5.0 * dt

    if engine.is_key_down("W"):
        engine.move_camera(fx * speed, 0.0, fz * speed)
    if engine.is_key_down("S"):
        engine.move_camera(-fx * speed, 0.0, -fz * speed)
    if engine.is_key_down("A"):
        engine.move_camera(-rx * speed, 0.0, -rz * speed)
    if engine.is_key_down("D"):
        engine.move_camera(rx * speed, 0.0, rz * speed)

    if engine.is_key_down("Space"):
        engine.move_camera(0.0, speed, 0.0)

    if engine.is_key_down("Left Shift"):
        engine.move_camera(0.0, -speed, 0.0)

    running = engine.update()

    time.sleep(dt)

engine.quit()
