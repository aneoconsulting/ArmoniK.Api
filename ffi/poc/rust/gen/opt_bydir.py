import sys, math, collections
def load(d):
    rows={}
    for l in open(d+"/variants-codec.tsv"):
        if l.startswith("#"): continue
        f=l.rstrip("\n").split("\t")
        if f[0]=="input": hdr=f; continue
        rows[(f[0],f[1],f[2])]={h:float(v) for h,v in zip(hdr[3:],f[3:]) if v}
    return rows
a,b=load(sys.argv[1]),load(sys.argv[2])
cols=sys.argv[3].split(",")
g=collections.defaultdict(list)
for k in a:
    if k not in b: continue
    inp,d,var=k
    vk=var.split("/")[0]
    for c in cols:
        if c in a[k] and c in b[k]:
            g[(d,vk,c)].append(b[k][c]/a[k][c])
for (d,vk,c),v in sorted(g.items()):
    print("%-12s %-22s %-14s n=%3d gmean %.3f  [%.2f, %.2f]"%(d,vk,c,len(v),math.exp(sum(map(math.log,v))/len(v)),min(v),max(v)))
