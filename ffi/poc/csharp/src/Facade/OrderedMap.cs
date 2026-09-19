// The facade's map, and the reason it is not a `SortedDictionary`.
//
// The canonical form of `ffi/schema/README.md` sorts map entries by key.
// `Google.Protobuf`'s `MapField` writes in INSERTION order and .NET's runtime
// has no deterministic-serialization switch (the C++ and Java runtimes do), so
// canonical bytes from the incumbent come from the host inserting sorted --
// which is what `BuildGp` does.
//
// A facade backed by a `SortedDictionary` would then be paying red-black
// inserts on decode while the incumbent pays hash inserts, and the managed
// decode column, which is the single most valuable number in this slice, would
// be a comparison of container types rather than of codecs. So the facade's map
// is what `MapField` is: a `Dictionary` for lookup and a list for order.

using System;
using System.Collections;
using System.Collections.Generic;

namespace Armonik.Ffi.Facade;

public sealed class OrderedMap<TKey, TValue> : IEnumerable<KeyValuePair<TKey, TValue>>
{
    private readonly Dictionary<TKey, int> _index;
    private readonly List<KeyValuePair<TKey, TValue>> _order;

    public OrderedMap()
    {
        _index = new Dictionary<TKey, int>();
        _order = new List<KeyValuePair<TKey, TValue>>();
    }

    public int Count => _order.Count;

    public TValue this[TKey k]
    {
        get => _order[_index[k]].Value;
        set
        {
            if (_index.TryGetValue(k, out int i)) _order[i] = new KeyValuePair<TKey, TValue>(k, value);
            else { _index[k] = _order.Count; _order.Add(new KeyValuePair<TKey, TValue>(k, value)); }
        }
    }

    public bool TryGetValue(TKey k, out TValue v)
    {
        if (_index.TryGetValue(k, out int i)) { v = _order[i].Value; return true; }
        v = default;
        return false;
    }

    public void Clear() { _index.Clear(); _order.Clear(); }

    /// Indexed so the encoder can walk entries without allocating an enumerator,
    /// which is what `MapField`'s own writer does through its linked list.
    public KeyValuePair<TKey, TValue> At(int i) => _order[i];

    public IEnumerator<KeyValuePair<TKey, TValue>> GetEnumerator() => _order.GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => _order.GetEnumerator();
}
