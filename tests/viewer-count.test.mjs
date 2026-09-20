import test from 'node:test';
import assert from 'node:assert/strict';
import { viewerCountLabel } from '../ui/view-model.js';

test('missing viewer count is hidden rather than presented as zero', () => {
  assert.equal(viewerCountLabel(null), '');
  assert.equal(viewerCountLabel(undefined), '');
});
test('zero is a known viewer count', () => assert.equal(viewerCountLabel({count:0,stale:false}), '0 viewers'));
test('one viewer uses singular grammar', () => assert.equal(viewerCountLabel({count:1,stale:false}), '1 viewer'));
test('viewer counts use readable grouping through the backend u32 range', () => {
  assert.equal(viewerCountLabel({count:1234,stale:false}), '1,234 viewers');
  assert.equal(viewerCountLabel({count:4294967295,stale:false}), '4,294,967,295 viewers');
});
test('stale counts remain explicitly qualified including zero and one', () => {
  assert.equal(viewerCountLabel({count:0,stale:true}), '0 viewers (stale)');
  assert.equal(viewerCountLabel({count:1,stale:true}), '1 viewer (stale)');
  assert.equal(viewerCountLabel({count:1234,stale:true}), '1,234 viewers (stale)');
});
test('invalid viewer counts are hidden', () => {
  for (const count of [-1,0.5,4294967296,NaN,Infinity,'12',null,undefined]) {
    assert.equal(viewerCountLabel({count,stale:false}), '');
  }
});
