import("stdfaust.lib");

myosc = os.lf_sawpos : *(ma.PI *2) : sin;

numosc = 10;
replicated = _<:par(i,numosc, myosc: /(max(i,1)) ):> _;
base = 50;
process = base : replicated ;
