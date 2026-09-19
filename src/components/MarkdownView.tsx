import Markdown from "react-markdown";

export function MarkdownView({ content }: { content: string }) {
  return (
    <div className="markdown-body">
      <Markdown
        components={{
          a: ({ children, href }) => (
            <a href={href} rel="noreferrer" target="_blank">
              {children}
            </a>
          ),
        }}
      >
        {content}
      </Markdown>
    </div>
  );
}
