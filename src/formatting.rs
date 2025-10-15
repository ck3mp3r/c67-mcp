use crate::client::SearchResponse;

#[must_use]
pub fn format_search_results(response: &SearchResponse) -> String {
    if response.results.is_empty() {
        return "No documentation libraries found matching your query.".to_string();
    }

    let formatted_results: Vec<String> = response
        .results
        .iter()
        .map(|result| {
            let mut parts = vec![
                format!("- Title: {}", result.title),
                format!("- Context7-compatible library ID: {}", result.id),
                format!("- Description: {}", result.description),
            ];

            if let Some(snippets) = result.total_snippets
                && snippets != -1
            {
                parts.push(format!("- Code Snippets: {snippets}"));
            }

            if let Some(trust_score) = result.trust_score
                && trust_score >= 0.0
            {
                parts.push(format!("- Trust Score: {trust_score:.1}"));
            }

            if let Some(versions) = &result.versions
                && !versions.is_empty()
            {
                parts.push(format!("- Versions: {}", versions.join(", ")));
            }

            parts.join("\n")
        })
        .collect();

    formatted_results.join("\n----------\n")
}
