// One body, included by odr_a.cpp (-std=c++11) and odr_b.cpp (-std=c++17). ONE source, so
// the only difference between the two translation units is the switch the consumer owns.
#include "odr.h"

static const AkOdrFact kFacts[] = {
#define F(n, v) {n, v},
    AK_ODR_FACTS(F)
#undef F
};

const AkOdrFact *AK_ODR_FN(odr_facts)(size_t *n) {
  *n = sizeof(kFacts) / sizeof(kFacts[0]);
  return kFacts;
}

long AK_ODR_FN(odr_std)() { return (long)__cplusplus; }

void AK_ODR_FN(odr_build)(shapes::TaskDetailed *t, shapes::Probe *p) {
  t->id = "the-id";
  t->status = shapes::TaskStatus(9);
  t->parent_task_ids.push_back("a");
  t->parent_task_ids.push_back("b");
  shapes::TaskOptions &o = t->options.emplace();
  o.max_retries = 7;
  o.options["k"] = "v";
  o.max_duration.emplace().seconds = 42;
  t->output.emplace().error = "boom";

  p->id = "probe";
  p->opt_count.set(0);         // present AND zero: the case a group cannot tell from absent
  p->opt_label.set(std::string());
  p->body.set_as_text() = "hello";
}

bool AK_ODR_FN(odr_read)(const shapes::TaskDetailed &t, const shapes::Probe &p) {
  if (t.id != "the-id") return false;
  if (t.status.v != 9) return false;
  if (t.parent_task_ids.size() != 2 || t.parent_task_ids[1] != "b") return false;
  if (!t.options.has_value() || t.options->max_retries != 7) return false;
  if (t.options->options.size() != 1 || t.options->options.find("k")->second != "v")
    return false;
  if (!t.options->max_duration.has_value() || t.options->max_duration->seconds != 42)
    return false;
  if (!t.output.has_value() || t.output->error != "boom") return false;
  if (p.id != "probe") return false;
  if (!p.opt_count.has_value() || *p.opt_count != 0) return false;
  if (!p.opt_label.has_value() || !p.opt_label->empty()) return false;
  if (p.body.which() != shapes::ProbeBody::kAsText) return false;
  if (p.body.as_text() != "hello") return false;
  return true;
}
