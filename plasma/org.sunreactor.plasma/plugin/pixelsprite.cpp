#include "pixelsprite.h"

#include <QImageReader>

PixelSprite::PixelSprite(QQuickItem *parent)
    : PixelRaster(parent)
{
}

void PixelSprite::setSource(const QUrl &source)
{
    if (m_source == source) {
        return;
    }
    m_source = source;
    reload();
}

void PixelSprite::setArtSize(int size)
{
    size = qBound(1, size, 256);
    if (m_artSize == size) {
        return;
    }
    m_artSize = size;
    reload();
}

void PixelSprite::reload()
{
    m_decoded = QImage();
    m_status = Null;
    if (!m_source.isEmpty()) {
        const QString path = m_source.isLocalFile() ? m_source.toLocalFile()
            : m_source.scheme() == QLatin1String("qrc") ? QLatin1Char(':') + m_source.path()
                                                         : m_source.toString();
        QImageReader reader(path);
        // Vector sources are rendered straight at the art grid; bitmaps are
        // expected at their art size already and only checked.
        reader.setScaledSize(QSize(m_artSize, m_artSize));
        const QImage image = reader.read();
        if (image.isNull()) {
            m_status = Error;
        } else {
            m_decoded = image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
            m_status = Ready;
        }
    }
    Q_EMIT spriteChanged();
    invalidate();
}
