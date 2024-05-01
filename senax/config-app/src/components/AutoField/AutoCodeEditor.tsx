import * as React from "react";
import { Controller } from "react-hook-form";
import FormField from "@cloudscape-design/components/form-field";
import CodeEditor from "@cloudscape-design/components/code-editor";
import "ace-builds/css/ace.css";
import "ace-builds/css/theme/dawn.css";
import "ace-builds/css/theme/tomorrow_night_bright.css";
import jsWorkerUrl from "ace-builds/src-min-noconflict/worker-javascript.js?url";

interface Props {
  name: string;
  path: string;
  form: any;
  definition: any;
  errors: object;
  label: string;
}

const i18nStrings = {
  loadingState: "Loading code editor",
  errorState: "There was an error loading the code editor.",
  errorStateRecovery: "Retry",

  editorGroupAriaLabel: "Code editor",
  statusBarGroupAriaLabel: "Status bar",

  cursorPosition: (row: any, column: any) => `Ln ${row}, Col ${column}`,
  errorsTab: "Errors",
  warningsTab: "Warnings",
  preferencesButtonAriaLabel: "Preferences",

  paneCloseButtonAriaLabel: "Close",

  preferencesModalHeader: "Preferences",
  preferencesModalCancel: "Cancel",
  preferencesModalConfirm: "Confirm",
  preferencesModalWrapLines: "Wrap lines",
  preferencesModalTheme: "Theme",
  preferencesModalLightThemes: "Light themes",
  preferencesModalDarkThemes: "Dark themes",
};

function AutoCodeEditor(props: Props) {
  const name = props.name;
  const path = props.path;
  const form = props.form;
  const errors = props.errors as any;
  const label = props.label;
  const [preferences, setPreferences] = React.useState({});
  const [loading, setLoading] = React.useState(true);
  const [ace, setAce] = React.useState();

  React.useEffect(() => {
    async function loadAce() {
      const ace = await import("ace-builds");
      await import("ace-builds/esm-resolver");
      ace.config.setModuleUrl("ace/mode/javascript_worker", jsWorkerUrl);
      ace.config.set("useStrictCSP", true);
      return ace;
    }
    loadAce()
      .then((ace) => setAce(ace as any))
      .finally(() => setLoading(false));
  }, []);
  if (form.getValues(`${path}${name}`) === undefined) {
    form.setValue(`${path}${name}`, props.definition.examples[0]);
  }

  return (
    <Controller
      name={`${path}${name}`}
      control={form.control}
      render={({ field }) => (
        <FormField
          stretch
          description={props.definition.description}
          label={label}
          errorText={errors[name]?.message}
        >
          <CodeEditor
            ace={ace}
            loading={loading}
            value={field.value || props.definition.examples[0]}
            language="javascript"
            onDelayedChange={({ detail }) =>
              form.setValue(
                `${path}${name}`,
                detail.value === "" || detail.value === undefined
                  ? null
                  : detail.value,
                { shouldDirty: true, shouldValidate: true },
              )
            }
            preferences={preferences}
            onPreferencesChange={(event) => setPreferences(event.detail)}
            i18nStrings={i18nStrings}
            ref={field.ref}
          />
        </FormField>
      )}
    />
  );
}

export default AutoCodeEditor;
