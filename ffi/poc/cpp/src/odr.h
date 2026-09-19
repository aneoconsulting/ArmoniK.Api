// README 5.1's hard stop, as a cross-translation-unit test rather than as a claim.
//
// `odr_a.cpp` is compiled at -std=c++11 and `odr_b.cpp` at -std=c++17. They are linked
// together, their layout-fact tables are compared elementwise, and facade objects are
// constructed in one and read in the other, both ways. A type whose layout moved with the
// standard level shows up as a differing row, not as a wrong field at run time.
#ifndef AK_ODR_CHECK_H
#define AK_ODR_CHECK_H
#include <cstddef>
#include "generated/types.h"
#include "generated/odr.h"

const AkOdrFact *odr_facts_11(size_t *n);
const AkOdrFact *odr_facts_17(size_t *n);
long odr_std_11();
long odr_std_17();

// Objects built in one TU and read in the other.
void odr_build_11(shapes::TaskDetailed *t, shapes::Probe *p);
void odr_build_17(shapes::TaskDetailed *t, shapes::Probe *p);
bool odr_read_11(const shapes::TaskDetailed &t, const shapes::Probe &p);
bool odr_read_17(const shapes::TaskDetailed &t, const shapes::Probe &p);
#endif
