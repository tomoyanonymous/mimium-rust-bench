import("stdfaust.lib");

myosc = os.lf_sawpos : *(ma.PI *2) : sin;

numosc = 100;
replicated = _<:par(i,numosc, *(i): myosc: /(max(i,1)) ):> _;
base = 50;
process = base : replicated ;
