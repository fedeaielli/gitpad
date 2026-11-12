// @vitest-environment happy-dom
import { describe, expect, it } from 'vitest'
import { odfToHtml } from './odf'

const CONTENT = `<?xml version="1.0"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0">
  <office:body>
    <office:text>
      <text:h text:outline-level="1">Title</text:h>
      <text:p>Plain <text:span text:style-name="Strong">bold</text:span></text:p>
      <text:list text:style-name="L1">
        <text:list-item><text:p>one</text:p></text:list-item>
        <text:list-item><text:p>two</text:p></text:list-item>
      </text:list>
      <table:table>
        <table:table-row>
          <table:table-cell><text:p>cell</text:p></table:table-cell>
        </table:table-row>
      </table:table>
    </office:text>
  </office:body>
</office:document-content>`

const STYLES = `<?xml version="1.0"?>
<office:document-styles
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="Strong">
      <style:text-properties fo:font-weight="bold"/>
    </style:style>
    <text:list-style style:name="L1">
      <text:list-level-style-number text:level="1"/>
    </text:list-style>
  </office:automatic-styles>
</office:document-styles>`

const AUTOMATIC_CONTENT = `<?xml version="1.0"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-weight="bold"/>
    </style:style>
    <text:list-style style:name="L1">
      <text:list-level-style-number text:level="1"/>
    </text:list-style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Plain <text:span text:style-name="T1">bold</text:span></text:p>
      <text:list text:style-name="L1">
        <text:list-item><text:p>one</text:p></text:list-item>
      </text:list>
    </office:text>
  </office:body>
</office:document-content>`

describe('odfToHtml', () => {
  it('converts headings, inline styles, lists and tables', () => {
    const html = odfToHtml(CONTENT, STYLES)
    expect(html).toContain('<h1>Title</h1>')
    expect(html).toContain('<strong>bold</strong>')
    expect(html).toContain('<ol>')
    expect(html).toContain('<li><p>one</p></li>')
    expect(html).toContain('<td><p>cell</p></td>')
  })

  it('reads the automatic styles LibreOffice writes in content.xml', () => {
    const html = odfToHtml(AUTOMATIC_CONTENT)
    expect(html).toContain('<strong>bold</strong>')
    expect(html).toContain('<ol>')
  })

  it('returns nothing when the body is missing', () => {
    expect(odfToHtml('<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"/>')).toBe('')
  })
})
