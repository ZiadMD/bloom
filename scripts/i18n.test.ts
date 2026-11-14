import { describe, expect, test } from "bun:test";
import { createTranslator } from "../src/i18n/engine";
import { resolveLanguage, normalizeLanguageSetting } from "../src/i18n/runtime";
import { sanitizeCatalog, validateCatalog, type Catalog } from "../src/i18n/catalog";

const english: Catalog = {
	settings: { title: "Settings", greeting: "Hello {name}" },
	items: { one: "{count} item", other: "{count} items" }
};

describe("partial translation safety", () => {
	for (const value of [undefined, null, "", "   ", 42, [], {}, "Hello {wrong}"]) {
		test(`invalid message ${JSON.stringify(value)} uses English`, () => {
			const tr = createTranslator(english, { fr: { settings: { greeting: value } } });
			tr.setLocale("fr");
			expect(tr.t("settings.greeting", { name: "Ada" })).toBe("Hello Ada");
			expect(tr.t("settings.title")).toBe("Settings");
		});
	}

	test("valid entries survive alongside invalid or absent siblings", () => {
		const tr = createTranslator(english, {
			fr: { settings: { title: "Paramètres", greeting: "" } }
		});
		tr.setLocale("fr");
		expect(tr.t("settings.title")).toBe("Paramètres");
		expect(tr.t("settings.greeting", { name: "Ada" })).toBe("Hello Ada");
		tr.setLocale("en");
		expect(tr.t("settings.title")).toBe("Settings");
	});

	test("interpolation preserves user text as text, including markup characters", () => {
		const tr = createTranslator(english, {});
		expect(tr.t("settings.greeting", { name: "<Ada & Co>" })).toBe("Hello <Ada & Co>");
	});

	test("unsupported locale safely uses bundled English", () => {
		const tr = createTranslator(english, {});
		tr.setLocale("zz");
		expect(tr.t("settings.title")).toBe("Settings");
	});

	test("unknown and inherited catalog properties cannot become messages", () => {
		const translated = Object.create({ title: "inherited" });
		translated.unknown = "unexpected";
		expect(sanitizeCatalog(english, translated)).toEqual({});
	});
});

describe("plurals", () => {
	test("empty or wholly invalid plural groups cannot mask English with an object", () => {
		for (const items of [{}, null, "invalid", { one: "" }, { one: "{wrong}" }, { other: 12 }]) {
			const tr = createTranslator(english, { ru: { items } });
			tr.setLocale("ru");
			expect(tr.tCount("items", 1)).toBe("1 item");
			expect(tr.tCount("items", 21)).toBe("21 items");
		}
	});
	test("missing Russian forms select English grammar using English rules", () => {
		const tr = createTranslator(english, { ru: {} });
		tr.setLocale("ru");
		for (const count of [0, 2, 5, 21, 22, 101])
			expect(tr.tCount("items", count)).toBe(`${count} items`);
		expect(tr.tCount("items", 1)).toBe("1 item");
	});

	test("additional Russian plural categories are retained", () => {
		const tr = createTranslator(english, {
			ru: {
				items: {
					one: "{count} предмет",
					few: "{count} предмета",
					many: "{count} предметов",
					other: "{count} предмета"
				}
			}
		});
		tr.setLocale("ru");
		expect(tr.tCount("items", 1)).toBe("1 предмет");
		expect(tr.tCount("items", 2)).toBe("2 предмета");
		expect(tr.tCount("items", 5)).toBe("5 предметов");
		expect(tr.tCount("items", 21)).toBe("21 предмет");
	});

	test("a partially translated plural group falls back per form", () => {
		const tr = createTranslator(english, { ru: { items: { few: "{count} предмета" } } });
		tr.setLocale("ru");
		expect(tr.tCount("items", 2)).toBe("2 предмета");
		expect(tr.tCount("items", 21)).toBe("21 items");
	});

	test("French zero uses French singular, missing French uses English plural", () => {
		const tr = createTranslator(english, { fr: { items: { one: "{count} élément" } } });
		tr.setLocale("fr");
		expect(tr.tCount("items", 0)).toBe("0 élément");
		expect(tr.tCount("items", 2)).toBe("2 items");
	});
});

describe("language settings", () => {
	test("regions remain distinct and exact matching ignores case", () => {
		expect(resolveLanguage("pt-br")).toBe("pt-BR");
		expect(resolveLanguage("pt_PT")).toBe("pt-PT");
		expect(resolveLanguage("pt-AO")).toBe("en");
		expect(resolveLanguage("pt")).toBe("en");
	});
	test("base matching and ordered system preferences", () => {
		expect(resolveLanguage("fr-CA")).toBe("fr");
		expect(resolveLanguage("system", ["de-DE", "pt-BR", "fr"])).toBe("pt-BR");
		expect(resolveLanguage(null, ["es-MX"])).toBe("es");
		expect(resolveLanguage("system", ["zz"])).toBe("en");
		expect(resolveLanguage("zz", ["fr"])).toBe("en");
	});
	test("removed and invalid settings normalize to System default", () => {
		for (const value of [undefined, null, "", "system"])
			expect(normalizeLanguageSetting(value)).toBe("system");
		for (const value of [1, "unknown", false]) expect(normalizeLanguageSetting(value)).toBe("en");
		expect(normalizeLanguageSetting("pt-br")).toBe("pt-BR");
	});
});

describe("catalog validation", () => {
	test("partial and empty languages do not block releases", () => {
		expect(validateCatalog(english, {}, "fr")).toEqual([]);
		expect(validateCatalog(english, { settings: { title: "Paramètres" } }, "fr")).toEqual([]);
	});
	test("invalid roots, unknown sections, and bad messages fail validation", () => {
		for (const target of [
			null,
			[],
			{ unknown: {} },
			{ settings: "wrong" },
			{ settings: { title: " " } },
			{ settings: { greeting: "Bonjour {wrong}" } }
		]) {
			expect(validateCatalog(english, target, "fr").length).toBeGreaterThan(0);
		}
	});
	test("valid additional plural categories pass; typos fail", () => {
		expect(
			validateCatalog(
				english,
				{ items: { few: "{count} предмета", many: "{count} предметов" } },
				"ru"
			)
		).toEqual([]);
		expect(validateCatalog(english, { items: { fwe: "{count} предмета" } }, "ru").length).toBe(1);
	});
	test("repeated placeholders have set semantics", () => {
		expect(
			validateCatalog(english, { settings: { greeting: "{name}, bonjour {name}" } }, "fr")
		).toEqual([]);
	});
});
