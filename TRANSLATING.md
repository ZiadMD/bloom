# Help translate Bloom

You can contribute a few phrases or a whole screen. You do not need to finish a language, install Bloom's development tools, or know how to code.

Bloom uses English wherever a translation is missing. New features can ship while translations catch up. Translations are bundled with releases and work offline.

## Easiest: send us the words

Open a [translation contribution](https://github.com/SehajveerSingh2005/bloom/issues/new?template=translation.yml). Tell us your language and, for each phrase, provide the English text and your translation. Mention which screen it appears on. A screenshot is helpful when the wording is ambiguous.

A maintainer will put the translations into the right file. You can also use this form to correct an existing translation or offer to review your language. Please specify Brazilian or European Portuguese.

## Edit a language file in your browser

If you are comfortable editing a small text file:

1. Find your file in [`src/i18n/locales`](src/i18n/locales). English (`en.json`) is the reference. French is `fr.json`, Spanish is `es.json`, Brazilian Portuguese is `pt-BR.json`, and European Portuguese is `pt-PT.json`.
2. Click GitHub's pencil button. GitHub will guide you through creating a fork if necessary.
3. Add the entries you translated, keeping their original keys and nesting. Leave untranslated entries out. The starter files can be just `{}`.
4. Choose **Propose changes**, then open a pull request explaining your language and which screens you translated. Automated checks will report formatting or placeholder mistakes; a maintainer can help fix them.

For example, this is a complete, valid partial French file:

```json
{
  "common": {
    "cancel": "Annuler"
  },
  "settings": {
    "title": "Paramètres"
  }
}
```

Every other message will still appear in English. Do not copy all the English text into your file: that makes it harder to see what still needs translation. JSON requires double quotes and no trailing comma after the last entry.

## Wording guidelines

- Translate the values, keeping the keys unchanged. Keep names such as Bloom and Windows unchanged.
- Keep placeholders exactly as written. For example, `Version {version}` must still contain `{version}`. You can move a placeholder to wherever it belongs in your language.
- Keep labels short: Bloom has small buttons, a compact notch, menus, and popovers. Translate whole messages naturally rather than word by word.
- Keep the tone clear and direct. Use consistent wording for Dock, Island/Notch, and Command Center. Explain terminology choices when opening your PR so other contributors can review them.
- Do not insert HTML. Messages are displayed as text. Preserve meaningful newlines.
- Do not submit blank values to hide text. Remove an unfinished entry so English is used.
- App names, song titles, filenames, network names, and device names come from Windows or the user. Those are not catalog translations. Technical details returned by Windows can also remain in their original language.

Plural messages are grouped by grammatical form. English usually has `one` and `other`; Russian also needs `few` and `many`. Use the forms your language needs and preserve `{count}` in every form. Partial plural groups are allowed: a missing form uses English grammar, not the selected language's English-incompatible plural rule.

## Try it in Bloom

In a build containing your changes, choose **Settings → General → Language**. The selection applies live to Settings, the notch, dock, overlays, and Bloom's native tray menu, and is remembered after restart. Check long labels, menus, keyboard-accessible names, and larger UI scales. Selecting an empty starter catalog intentionally displays English until translations are added.

Language files are included in a normal release; editing a file on GitHub does not change an already installed build.

## Maintainer and developer notes

The app uses i18next/react-i18next behind the helpers in `src/i18n`. English is the source of truth. Runtime sanitization discards unknown entries, empty/whitespace text, invalid types, and messages with incorrect placeholders before the engine resolves per-message English fallback. Catalogs are bundled locally; no translation account, service, credential, or network request is required.

```powershell
bun run i18n:check
bun run i18n:check --missing
bun run i18n:test
bun run build
```

The checker reports coverage without failing on missing translations. Malformed JSON, unknown keys, invalid values, placeholder mismatches, and literal calls missing English messages fail CI. Coverage compares English entries; extra plural forms do not inflate it. Dynamic message-key expressions still require code review.

For new UI, add a complete English message and use `t("semantic.messageKey", { name })`. Translate complete sentences, not concatenated fragments, so word order can change. For counts use `tCount("semantic.pluralGroup", count)`, with `one`/`other` English source entries. Use stable identifiers for program behavior; do not compare translated labels. Components rendering translations must call `useTranslation()` so changes also update memoized UI. Add the locale to memo dependencies when translated output is cached.

Do not require contributors to update every language for a new feature. When English meaning changes, use a new semantic key or explicitly request review of existing translations; the validator cannot judge whether wording is still accurate.

To add another language, create a partial JSON catalog, import/register it in `src/i18n/runtime.ts`, and add its bundled catalog to `src-tauri/src/i18n.rs` for the native tray. Use canonical locale codes and native display names. Regional variants are distinct; an unsupported Portuguese region does not silently select Brazilian or European Portuguese. Add the relevant fallback/locale tests. The currently registered languages use left-to-right layout; right-to-left languages require a separate layout pass before being offered in the picker.

Before release run the normal frontend/Rust gates and test a deliberately partial catalog on Windows. Include startup, live switching, external edits/removal of `bloom-language`, import/reset, saved-language restart, native tray, long strings, and larger UI scale. These cases matter beyond checking the JSON files.

If translation volume later justifies Crowdin or another service, import these same catalogs and let it propose reviewed PRs. The application and English fallback do not depend on that choice. Contributions remain covered by Bloom's GPL-3.0 license.
