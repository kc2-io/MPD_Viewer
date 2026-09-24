import test from 'node:test';
import assert from 'node:assert/strict';
import { timerLabel } from '../ui/view-model.js';
test('assignment timer labels preserve meaning at countdown boundaries',()=>{
  assert.equal(timerLabel(null),'Always · no assignment timer');
  for(const [seconds,clock] of [[0,'0:00'],[1,'0:01'],[59,'0:59'],[60,'1:00'],[600,'10:00'],[86400,'1440:00']]) {
    assert.equal(timerLabel({state:'counting',remaining_seconds:seconds}),`${clock} assigned time left`);
    assert.equal(timerLabel({state:'automation_paused',remaining_seconds:seconds}),`Automation paused · ${clock} assigned time left`);
  }
  assert.equal(timerLabel({state:'waiting_for_alternative'}),'Time reached · waiting for another live channel');
  assert.equal(timerLabel({state:'closing_for_rotation'}),'Timer reached · rotating to next live channel');
});
