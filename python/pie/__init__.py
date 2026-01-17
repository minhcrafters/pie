__all__ = ["engine", "scene", "audio"]

# Try to import the compiled extension module that maturin copies into this
# package (e.g. `pie.cp311-win_amd64.pyd` as the `pie` submodule). When the
# extension initializes it registers `pie.engine` and `pie.scene` in
# `sys.modules`, which allows lazy imports below to work.
try:
	import importlib

	# Import the extension as a submodule (relative import of the package's
	# own name). This will be `pie.pie` and will load the compiled binary
	# located in this package directory when present.
	importlib.import_module(f".{__name__.split('.')[-1]}", package=__name__)
except Exception:
	# Ignore: the compiled extension may not be present in editable/dev
	# environments yet; fallback to lazy imports.
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
