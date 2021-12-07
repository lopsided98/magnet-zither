{ lib, buildPythonPackage, fetchFromGitHub, setuptools, lxml, gdb }:

buildPythonPackage {
  pname = "cmdebug";
  version = "unstable-20200310";
  
  src = fetchFromGitHub {
    owner = "bnahill";
    repo = "PyCortexMDebug";
    rev = "77e9717a0e7c5b44214bdf70fbddf376cf2e8a7d";
    sha256 = "07y58s79wszim19kvcjsnvp6s73488cm47bhzxljb72knx1pb9qr";
  };

  doCheck = false;

  propagatedBuildInputs = [ setuptools lxml ];
  checkInputs = [ gdb ];
  
  meta = with lib; {
    description = "A set of GDB/Python-based utilities to make life debugging ARM Cortex-M processors a bit easier";
    maintainers = with maintainers; [ lopsided98 ];
  };
}
