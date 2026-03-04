Run the conversation verification E2E test suite.

Parse the arguments in `$ARGUMENTS` to determine which tests to run:

- If `$ARGUMENTS` contains `--mic`: run only mic-only tests with `-g "Verify: Mic-only"`
- If `$ARGUMENTS` contains `--tab`: run only tab audio tests with `-g "Verify: Tab audio"`
- If `$ARGUMENTS` contains `--full`: run only full conversation tests with `-g "Verify: Full meeting"`
- If `$ARGUMENTS` contains `--interim`: run only interim handling tests with `-g "Verify: Interim"`
- If `$ARGUMENTS` is empty or contains no filter flags: run all verification tests

If `$ARGUMENTS` contains `--headed`: add `--headed` to the Playwright command.

Run the tests from the `web-app` directory:

```
cd web-app && npx playwright test e2e/09-verify-conversation.spec.ts --reporter=list [flags]
```

After the tests complete, report:
- Total tests run and passed/failed count
- Any failing test names with a brief error description
- If all pass, confirm the verification suite is green
