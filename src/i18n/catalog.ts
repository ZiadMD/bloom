export type Catalog = { [key: string]: string | Catalog };
const categories = new Set(["zero", "one", "two", "few", "many", "other"]);

export function isObject(value: unknown): value is Record<string, unknown> {
	return value !== null && typeof value === "object" && !Array.isArray(value);
}

export function placeholders(value: string): string[] {
	return [...new Set([...value.matchAll(/\{(\w+)\}/g)].map((match) => match[1]))].sort();
}

export function validMessage(value: unknown, source: string): value is string {
	return (
		typeof value === "string" &&
		value.trim().length > 0 &&
		placeholders(value).join(",") === placeholders(source).join(",")
	);
}

/** Only known, structurally valid messages reach the translation engine. */
export function sanitizeCatalog(source: Catalog, target: unknown): Catalog {
	const result: Catalog = {};
	if (!isObject(target)) return result;
	for (const [key, value] of Object.entries(target)) {
		if (["__proto__", "prototype", "constructor"].includes(key)) continue;
		const reference = Object.prototype.hasOwnProperty.call(source, key)
			? source[key]
			: categories.has(key) && typeof source.other === "string"
				? source.other
				: undefined;
		if (typeof reference === "string") {
			if (validMessage(value, reference)) result[key] = value;
		} else if (isObject(reference)) {
			const child = sanitizeCatalog(reference as Catalog, value);
			if (Object.keys(child).length > 0) result[key] = child;
		}
	}
	return result;
}

/** Contributor files use nested plural groups; i18next uses suffixes. */
export function engineCatalog(catalog: Catalog): Catalog {
	const result: Catalog = {};
	for (const [key, value] of Object.entries(catalog)) {
		if (typeof value === "string") result[key] = value;
		else if (
			Object.keys(value).length > 0 &&
			Object.keys(value).every((k) => categories.has(k)) &&
			Object.values(value).every((v) => typeof v === "string")
		) {
			for (const [category, message] of Object.entries(value))
				result[`${key}_${category}`] = message;
		} else result[key] = engineCatalog(value);
	}
	return result;
}

export function flatten(catalog: unknown, prefix = "", result = new Map<string, unknown>()) {
	if (!isObject(catalog)) {
		result.set(prefix, catalog);
		return result;
	}
	for (const [key, value] of Object.entries(catalog)) {
		const path = prefix ? `${prefix}.${key}` : key;
		if (isObject(value)) flatten(value, path, result);
		else result.set(path, value);
	}
	return result;
}

/** Incomplete catalogs are valid; malformed entries are actionable CI errors. */
export function validateCatalog(source: Catalog, target: unknown, locale: string): string[] {
	const issues: string[] = [];
	const allowedCategories = new Set<string>([
		...new Intl.PluralRules(locale).resolvedOptions().pluralCategories,
		"zero"
	]);
	function visit(reference: Catalog, value: unknown, prefix: string) {
		if (!isObject(value)) {
			issues.push(`${prefix || "catalog"}: expected an object`);
			return;
		}
		for (const [key, translated] of Object.entries(value)) {
			const path = prefix ? `${prefix}.${key}` : key;
			const expected = Object.prototype.hasOwnProperty.call(reference, key)
				? reference[key]
				: allowedCategories.has(key) && typeof reference.other === "string"
					? reference.other
					: undefined;
			if (["__proto__", "prototype", "constructor"].includes(key) || expected === undefined) {
				issues.push(`${path}: unknown key`);
			} else if (typeof expected === "string") {
				if (!validMessage(translated, expected))
					issues.push(
						`${path}: expected nonempty text with placeholders [${placeholders(expected).join(", ")}]`
					);
			} else visit(expected, translated, path);
		}
	}
	visit(source, target, "");
	return issues;
}
