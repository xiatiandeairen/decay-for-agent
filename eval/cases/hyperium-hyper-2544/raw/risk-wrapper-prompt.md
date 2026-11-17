# Task

You are analyzing risk from machine-built decay risk wrappers that include honest_review.
Use only the JSON bundle below. Do not use external knowledge or web search.
Return JSON only, matching honest-review-analysis.v1.
For each wrapper with honest_review, preserve the same mechanism/impact/control statuses or lower them.
Every supported_by item must be copied exactly from honest_review observed or inferred.
Every honest_review unknown must remain visible in limits or uncertainties.
Copy all honest_review overall.non_claims into the output non_claims.
Do not claim a bug, do not prove complete coverage, and do not give a merge recommendation.

# Bundle
```json
{
  "case_id": "hyperium-hyper-2544",
  "git_context": {
    "base": "6a6a24030ec53321ce026fe294c56c86f21e19d4",
    "base_input": "6a6a24030ec53321ce026fe294c56c86f21e19d4",
    "changed_files": 2,
    "head": "093319d3e21f5ae3d4c5690466aec7bd9de4cb0c",
    "repo_root": "/private/tmp/decay-pr-eval/hyperium-hyper-2544/repo"
  },
  "pr_context": {
    "body": "Initial implementation that aims to solve #2534 by returning a `Parse::Internal` error instead of panic.  \r\n\r\nI intentionally added a new Parse error instead of a new Kind since i think this is still relevant and the diff is much cleaner.\r\ncc @nox  for [this message](https://github.com/hyperium/hyper/issues/2534#issuecomment-835436759), if this is very important to add here, let me know.\r\n\r\n\r\n\r\n",
    "comments": [
      {
        "author": {
          "login": "bensadiku"
        },
        "authorAssociation": "CONTRIBUTOR",
        "body": "@seanmonstar can you approve the ci? ",
        "createdAt": "2021-06-04T17:18:13Z",
        "id": "MDEyOklzc3VlQ29tbWVudDg1NDg4NTg0OQ==",
        "includesCreatedEdit": false,
        "isMinimized": false,
        "minimizedReason": "",
        "reactionGroups": [],
        "url": "https://github.com/hyperium/hyper/pull/2544#issuecomment-854885849",
        "viewerDidAuthor": false
      },
      {
        "author": {
          "login": "seanmonstar"
        },
        "authorAssociation": "MEMBER",
        "body": "Thanks for carrying this through!",
        "createdAt": "2021-06-04T22:18:01Z",
        "id": "MDEyOklzc3VlQ29tbWVudDg1NTA0MTQ1Nw==",
        "includesCreatedEdit": false,
        "isMinimized": false,
        "minimizedReason": "",
        "reactionGroups": [
          {
            "content": "HOORAY",
            "users": {
              "totalCount": 1
            }
          }
        ],
        "url": "https://github.com/hyperium/hyper/pull/2544#issuecomment-855041457",
        "viewerDidAuthor": false
      }
    ],
    "files": [
      {
        "additions": 5,
        "changeType": "MODIFIED",
        "deletions": 0,
        "path": "src/error.rs"
      },
      {
        "additions": 15,
        "changeType": "MODIFIED",
        "deletions": 27,
        "path": "src/proto/h1/role.rs"
      }
    ],
    "number": 2544,
    "title": "Return Parse::Internal error if there's an illegal header name or value",
    "url": "https://github.com/hyperium/hyper/pull/2544"
  },
  "version": "risk-context-wrapper.v1",
  "wrappers": [
    {
      "analysis_contract": {
        "consumer": "codex-cli",
        "expected_output": "semantic-confidence-evidence.v1",
        "rules": [
          "Use only evidence and context in this wrapper.",
          "Treat primary_evidence as direct evidence.",
          "Treat supporting_evidence as supporting only.",
          "Treat context_evidence as background only.",
          "Do not claim a bug unless the wrapper contains explicit external confirmation.",
          "Mention uncertainty when diff, source, test, or history context is missing."
        ],
        "task": "Produce semantic confidence evidence from this wrapper.",
        "wrapper_type": "ErrorPropagationWrapper"
      },
      "context_evidence": [
        {
          "category": "complexity",
          "confidence": "low",
          "file": "src/proto/h1/role.rs",
          "function": "src/proto/h1/role.rs:parse",
          "id": 2,
          "kind": "touched_existing_risky",
          "source": "diff_hunk+ast_snapshot",
          "subject": "function",
          "value": "context"
        }
      ],
      "coverage_limits": [],
      "diff_context": {
        "files": [
          {
            "hunks": [
              {
                "header": "@@ -1,151 +1,140 @@",
                "lines": [
                  " // `mem::uninitialized` replaced with `mem::MaybeUninit`,",
                  " // can't upgrade yet",
                  " #![allow(deprecated)]",
                  " ",
                  " use std::fmt::{self, Write};",
                  " use std::mem;",
                  " ",
                  " #[cfg(any(test, feature = \"server\", feature = \"ffi\"))]",
                  " use bytes::Bytes;",
                  " use bytes::BytesMut;",
                  "-use http::header::{self, Entry, HeaderName, HeaderValue};",
                  " #[cfg(feature = \"server\")]",
                  " use http::header::ValueIter;",
                  "+use http::header::{self, Entry, HeaderName, HeaderValue};",
                  " use http::{HeaderMap, Method, StatusCode, Version};",
                  " ",
                  " use crate::body::DecodedLength;",
                  " #[cfg(feature = \"server\")]",
                  " use crate::common::date;",
                  " use crate::error::Parse;",
                  " use crate::ext::HeaderCaseMap;",
                  " use crate::headers;",
                  " use crate::proto::h1::{",
                  "     Encode, Encoder, Http1Transaction, ParseContext, ParseResult, ParsedMessage,",
                  " };",
                  " use crate::proto::{BodyLength, MessageHead, RequestHead, RequestLine};",
                  " ",
                  " const MAX_HEADERS: usize = 100;",
                  " const AVERAGE_HEADER_SIZE: usize = 30; // totally scientific",
                  " ",
                  " macro_rules! header_name {",
                  "     ($bytes:expr) => {{",
                  "-        #[cfg(debug_assertions)]",
                  "         {",
                  "             match HeaderName::from_bytes($bytes) {",
                  "                 Ok(name) => name,",
                  "-                Err(_) => panic!(",
                  "-                    \"illegal header name from httparse: {:?}\",",
                  "-                    ::bytes::Bytes::copy_from_slice($bytes)",
                  "-                ),",
                  "-            }",
                  "-        }",
                  "-",
                  "-        #[cfg(not(debug_assertions))]",
                  "-        {",
                  "-            match HeaderName::from_bytes($bytes) {",
                  "-                Ok(name) => name,",
                  "-                Err(_) => panic!(\"illegal header name from httparse: {:?}\", $bytes),",
                  "+                Err(e) => maybe_panic!(e),",
                  "             }",
                  "         }",
                  "     }};",
                  " }",
                  " ",
                  " macro_rules! header_value {",
                  "     ($bytes:expr) => {{",
                  "-        #[cfg(debug_assertions)]",
                  "         {",
                  "-            let __hvb: ::bytes::Bytes = $bytes;",
                  "-            match HeaderValue::from_maybe_shared(__hvb.clone()) {",
                  "-                Ok(name) => name,",
                  "-                Err(_) => panic!(\"illegal header value from httparse: {:?}\", __hvb),",
                  "-            }",
                  "-        }",
                  "-",
                  "-        #[cfg(not(debug_assertions))]",
                  "-        {",
                  "-            // Unsafe: httparse already validated header value",
                  "             unsafe { HeaderValue::from_maybe_shared_unchecked($bytes) }",
                  "         }",
                  "     }};",
                  " }",
                  " ",
                  "+macro_rules! maybe_panic {",
                  "+    ($($arg:tt)*) => ({",
                  "+        let _err = ($($arg)*);",
                  "+        if cfg!(debug_assertions) {",
                  "+            panic!(\"{:?}\", _err);",
                  "+        } else {",
                  "+            error!(\"Internal Hyper error, please report {:?}\", _err);",
                  "+            return Err(Parse::Internal)",
                  "+        }",
                  "+    })",
                  "+}",
                  "+",
                  " pub(super) fn parse_headers<T>(",
                  "     bytes: &mut BytesMut,",
                  "     ctx: ParseContext<'_>,",
                  " ) -> ParseResult<T::Incoming>",
                  " where",
                  "     T: Http1Transaction,",
                  " {",
                  "     // If the buffer is empty, don't bother entering the span, it's just noise.",
                  "     if bytes.is_empty() {",
                  "         return Ok(None);",
                  "     }",
                  " ",
                  "     let span = trace_span!(\"parse_headers\");",
                  "     let _s = span.enter();",
                  "     T::parse(bytes, ctx)",
                  " }",
                  " ",
                  " pub(super) fn encode_headers<T>(",
                  "     enc: Encode<'_, T::Outgoing>,",
                  "     dst: &mut Vec<u8>,",
                  " ) -> crate::Result<Encoder>",
                  " where",
                  "     T: Http1Transaction,",
                  " {",
                  "     let span = trace_span!(\"encode_headers\");",
                  "     let _s = span.enter();",
                  "     T::encode(enc, dst)",
                  " }",
                  " ",
                  " // There are 2 main roles, Client and Server.",
                  " ",
                  " #[cfg(feature = \"client\")]",
                  " pub(crate) enum Client {}",
                  " ",
                  " #[cfg(feature = \"server\")]",
                  " pub(crate) enum Server {}",
                  " ",
                  " #[cfg(feature = \"server\")]",
                  " impl Http1Transaction for Server {",
                  "     type Incoming = RequestLine;",
                  "     type Outgoing = StatusCode;",
                  "     const LOG: &'static str = \"{role=server}\";",
                  " ",
                  "     fn parse(buf: &mut BytesMut, ctx: ParseContext<'_>) -> ParseResult<RequestLine> {",
                  "         debug_assert!(!buf.is_empty(), \"parse called with empty buf\");",
                  " ",
                  "         let mut keep_alive;",
                  "         let is_http_11;",
                  "         let subject;",
                  "         let version;",
                  "         let len;",
                  "         let headers_len;",
                  " ",
                  "         // Unsafe: both headers_indices and headers are using uninitialized memory,",
                  "         // but we *never* read any of it until after httparse has assigned",
                  "         // values into it. By not zeroing out the stack memory, this saves",
                  "         // a good ~5% on pipeline benchmarks.",
                  "         let mut headers_indices: [HeaderIndices; MAX_HEADERS] = unsafe { mem::uninitialized() };",
                  "         {",
                  "             let mut headers: [httparse::Header<'_>; MAX_HEADERS] = unsafe { mem::uninitialized() };",
                  "             trace!(",
                  "                 \"Request.parse([Header; {}], [u8; {}])\",",
                  "                 headers.len(),",
                  "                 buf.len()",
                  "             );",
                  "             let mut req = httparse::Request::new(&mut headers);",
                  "             let bytes = buf.as_ref();",
                  "             match req.parse(bytes) {",
                  "                 Ok(httparse::Status::Complete(parsed_len)) => {",
                  "                     trace!(\"Request.parse Complete({})\", parsed_len);",
                  "                     len = parsed_len;",
                  "                     subject = RequestLine(",
                  "                         Method::from_bytes(req.method.unwrap().as_bytes())?,",
                  "                         req.path.unwrap().parse()?,",
                  "                     );",
                  "                     version = if req.version.unwrap() == 1 {",
                  "                         keep_alive = true;",
                  "                         is_http_11 = true;",
                  "                         Version::HTTP_11",
                  "                     } else {"
                ],
                "reason": "linked_to_primary_evidence"
              }
            ],
            "path": "src/proto/h1/role.rs"
          }
        ]
      },
      "honest_review": {
        "dimensions": {
          "control": {
            "inferred": [],
            "observed": [],
            "status": "unknown",
            "unknown": [
              "no related test hunk was included in the wrapper"
            ]
          },
          "impact": {
            "inferred": [
              "PR metadata is consistent with a malformed-input behavior fix"
            ],
            "observed": [
              "changed production file: src/proto/h1/role.rs",
              "PR title: Return Parse::Internal error if there's an illegal header name or value",
              "PR body: Initial implementation that aims to solve #2534 by returning a `Parse::Internal` error instead of panic.  \r\n\r\nI intentionally added a new Parse error instead of a new Kind since i think this is still relevant and the diff is much cleaner.\r\ncc @nox  for [this message](https://github.com/hyperium/hyper/issues/2534#issuecomment-835436759), if this is very important to add here, let me know.\r\n\r\n\r\n\r\n"
            ],
            "status": "partial",
            "unknown": [
              "full public API contract is not available in this wrapper",
              "call graph from changed function to every caller is not available in this wrapper"
            ]
          },
          "mechanism": {
            "inferred": [
              "error propagation behavior changed in the same diff hunk"
            ],
            "observed": [
              "removed panic-like error handling: code contains `Err(_) => panic!(`",
              "added explicit error return: code contains `Err(e) => maybe_panic!(e)`",
              "removed error-flow behavior: code contains `Err(_) => panic!(`",
              "added error-flow behavior: code contains `Err(e) => maybe_panic!(e)`"
            ],
            "status": "partial",
            "unknown": [
              "whether this changes caller-visible behavior is not proven by the wrapper alone",
              "whether this is a fix, compatibility change, or risk introduction is not proven"
            ]
          }
        },
        "overall": {
          "control_status": "unknown",
          "impact_status": "partial",
          "judgement": "needs_more_context_before_review_claim",
          "mechanism_status": "partial",
          "non_claims": [
            "not evidence that this PR introduced a bug",
            "not proof of complete test coverage",
            "not a merge recommendation"
          ]
        },
        "risk_item": "error_propagation_change",
        "version": "honest-review.v1"
      },
      "primary_evidence": [
        {
          "category": "semantic",
          "confidence": "medium",
          "file": "src/proto/h1/role.rs",
          "id": 8,
          "kind": "error_propagation_changed",
          "source": "semantic_diff",
          "subject": "Err(e) => maybe_panic!(e)",
          "value": "direct"
        }
      ],
      "risk_question": "Did this diff change error propagation or terminal error handling?",
      "supporting_evidence": [],
      "wrapper_context": {
        "after_error_flow_lines": [
          "+                Err(e) => maybe_panic!(e),",
          "+            error!(\"Internal Hyper error, please report {:?}\", _err);",
          "+            return Err(Parse::Internal)"
        ],
        "after_error_lines": [
          "+                Err(e) => maybe_panic!(e),",
          "+            error!(\"Internal Hyper error, please report {:?}\", _err);",
          "+            return Err(Parse::Internal)"
        ],
        "before_error_flow_lines": [
          "-                Err(_) => panic!(",
          "-                Err(_) => panic!(\"illegal header name from httparse: {:?}\", $bytes),",
          "-                Err(_) => panic!(\"illegal header value from httparse: {:?}\", __hvb),"
        ],
        "before_panic_lines": [
          "-                Err(_) => panic!(",
          "-                Err(_) => panic!(\"illegal header name from httparse: {:?}\", $bytes),",
          "-                Err(_) => panic!(\"illegal header value from httparse: {:?}\", __hvb),"
        ],
        "error_tokens": [
          "maybe_panic"
        ],
        "primary_kind": "error_propagation_changed",
        "related_test_lines": [],
        "shared_input_tokens": []
      },
      "wrapper_id": "w1",
      "wrapper_type": "ErrorPropagationWrapper"
    }
  ]
}
```
