// Javascript unit tests.
"use strict";

// http://api.qunitjs.com/category/assert/
test("escapeHtml", function()
{
	equal(escapeHtml("hello world"), "hello world");
	equal(escapeHtml(">x<"), "&gt;x&lt;");
});

test("interval_to_time", function()
{
	strictEqual(interval_to_time(1), "1 millisecond");
	strictEqual(interval_to_time(50), "50 milliseconds");
});
