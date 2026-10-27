"""keydex — acceso tipado y explicable a configuración.

El motor vive en Rust; esta capa expone la API pública.
"""

from keydex._keydex import KeydexError, Settings, load

__all__ = ["KeydexError", "Settings", "load", "__version__"]
__version__ = "0.0.1"
