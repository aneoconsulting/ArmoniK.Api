package ak;

/**
 * The reverse-call surface, as an interface.
 *
 * <p>Two facades mean two {@code Binding} classes -- the owning one and open decision 13's
 * borrowed one -- and a {@code jmethodID} is obtained from a class. Resolving it against
 * one of them and calling it on an instance of the other is undefined, and the kind of
 * thing that works until it does not. Binding the ids to this interface instead makes
 * one shim serve both, which is also what lets the C half stay ungenerated.
 */
public interface Callbacks {
  /** ABI v1 section 6: the host drives iteration over its own containers. */
  int encLoop(long ctx, int slot, long token);

  /** ABI v1 7.1, the push family: the group of a message that has been parsed. */
  void decApply(long ctx, int slot, long fix);

  /** A non-batchable element: `new` then `apply`, because a repeated field inside the
   *  element needs the element to exist before its runs can be attached (7.2). */
  long decNew(long ctx, int slot);

  void decApplyElem(long ctx, int slot, long token, long fix);

  /** A run. Append; never size to the count you were handed (7.4). */
  void decAdd(long ctx, int slot, long token, long p, int n);
}
