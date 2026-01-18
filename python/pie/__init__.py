__all__ = ["engine", "scene", "audio"]

try:
    import importlib

    importlib.import_module(f".{__name__.split('.')[-1]}", package=__name__)
except Exception:
    pass


def __getattr__(name: str):
    if name in __all__:
        import importlib

        module = importlib.import_module(f"{__name__}.{name}")
        globals()[name] = module
        return module
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


def __dir__():
    return sorted(list(globals().keys()) + __all__)
