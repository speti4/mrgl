"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const Voice = require("../../ui/voice.js");

test("a message with a murloc line gives two lines", () => {
  assert.deepEqual(Voice.lines({ murloc: "Mrglglglgl!", plain: "2 addons updated" }), ["Mrglglglgl!", "2 addons updated"]);
});

test("a serious message gives the plain line alone", () => {
  assert.deepEqual(Voice.lines({ murloc: "", plain: "2 addons updated" }), ["2 addons updated"]);
});

test("a missing message gives one empty line, never a crash", () => {
  assert.deepEqual(Voice.lines(undefined), [""]);
});

test("one line joins with a dash", () => {
  assert.equal(Voice.oneLine({ murloc: "Mrglglgl.", plain: "mrgl 0.1.0" }), "Mrglglgl. — mrgl 0.1.0");
  assert.equal(Voice.oneLine({ murloc: "", plain: "mrgl 0.1.0" }), "mrgl 0.1.0");
});

test("every kind reaches the status bar", () => {
  for (const kind of Voice.KINDS) assert.equal(Voice.placement(kind).status, true, kind);
  assert.equal(Voice.placement("something else").status, true);
});

test("a finished message also toasts; the quiet kinds never do", () => {
  assert.deepEqual(Voice.placement("success"), { status: true, toast: true, sticky: false });
  assert.deepEqual(Voice.placement("warning"), { status: true, toast: true, sticky: false });
  for (const kind of ["banner", "progress", "nothing", "something else", undefined]) {
    assert.equal(Voice.placement(kind).toast, false, String(kind));
  }
});

test("an error or a fatal toast stays until dismissed", () => {
  assert.deepEqual(Voice.placement("error"), { status: true, toast: true, sticky: true });
  assert.deepEqual(Voice.placement("fatal"), { status: true, toast: true, sticky: true });
});

test("a kind has a class and an unknown one draws plain", () => {
  assert.equal(Voice.kindClass("error"), "kind-error");
  assert.equal(Voice.kindClass("banner"), "kind-banner");
  assert.equal(Voice.kindClass("murloc"), "kind-plain");
  assert.equal(Voice.kindClass(undefined), "kind-plain");
});

test("serious drops the murloc line and keeps the kind and the facts", () => {
  const message = { kind: "error", murloc: "Rwlrwlrwlrwl?!", plain: "addon not found: Questie (flavor forever)" };
  assert.deepEqual(Voice.serious(message), { kind: "error", murloc: "", plain: "addon not found: Questie (flavor forever)" });
  assert.deepEqual(Voice.serious(undefined), { kind: undefined, murloc: "", plain: "" });
});

// The toast cap (task 035): `pushedOut` takes the stack oldest first, the newest last, and names the toasts
// the cap pushes out.
const brief = (name) => ({ name, sticky: false });
const sticky = (name) => ({ name, sticky: true });
const names = (toasts) => toasts.map((toast) => toast.name);

test("the cap is three visible toasts, a constant", () => {
  assert.equal(Voice.TOAST_CAP, 3);
});

test("a stack within the cap loses nothing", () => {
  assert.deepEqual(Voice.pushedOut([]), []);
  assert.deepEqual(Voice.pushedOut(undefined), []);
  assert.deepEqual(Voice.pushedOut([brief("a"), brief("b"), brief("c")]), []);
  assert.deepEqual(Voice.pushedOut([sticky("a"), sticky("b"), sticky("c")]), []);
});

test("brief toasts go oldest first, as many as the cap needs", () => {
  const stack = [brief("a"), brief("b"), brief("c"), brief("d")];
  assert.deepEqual(names(Voice.pushedOut(stack)), ["a"]);
  assert.deepEqual(names(Voice.pushedOut([...stack, brief("e")])), ["a", "b"]);
  assert.deepEqual(names(Voice.pushedOut(stack, 2)), ["a", "b"]);
});

test("a sticky toast is never pushed out by a brief one", () => {
  assert.deepEqual(names(Voice.pushedOut([sticky("error"), brief("b"), brief("c"), brief("d")])), ["b"]);
  assert.deepEqual(names(Voice.pushedOut([brief("a"), sticky("error"), brief("c"), brief("d")])), ["a"]);
  assert.deepEqual(names(Voice.pushedOut([brief("a"), brief("b"), sticky("error"), brief("d")])), ["a"]);
});

test("the newest toast always shows, even over a stack of sticky ones", () => {
  assert.deepEqual(Voice.pushedOut([sticky("a"), sticky("b"), sticky("c"), brief("d")]), []);
  assert.deepEqual(Voice.pushedOut([sticky("a"), sticky("b"), sticky("c"), sticky("d")]), []);
});

test("ten quick messages leave three toasts with the sticky error on top", () => {
  const stack = [sticky("error")];
  for (let index = 1; index < 10; index += 1) {
    stack.push(brief(`success ${index}`));
    for (const old of Voice.pushedOut(stack)) stack.splice(stack.indexOf(old), 1);
  }
  assert.deepEqual(names(stack), ["error", "success 8", "success 9"]);
});
