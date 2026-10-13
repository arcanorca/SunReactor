#pragma once

#include <QUrl>
#include <QtQml/qqmlregistration.h>

#include "pixelitems.h"

/*!
 * A pixel-art sprite (SVG on a pixel grid, or PNG) rasterised at exactly its
 * art size and shown with nearest filtering, `artScale` logical px per art px.
 *
 * QML `Image` cannot do this at fractional device pixel ratios: it renders an
 * SVG at sourceSize x dpr and then resamples, so a 16-grid sprite at 1.35
 * comes out with uneven blocks. Here the image is decoded at `artSize` x
 * `artSize` (QImageReader scales vector sources to that size) and every art
 * pixel becomes a block of identical device pixels when `artScale * dpr` is a
 * whole number.
 */
class PixelSkySprite : public PixelRaster
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QUrl source READ source WRITE setSource NOTIFY spriteChanged)
    /*! Art pixels per side the source is decoded at (the sprite's grid). */
    Q_PROPERTY(int artSize READ artSize WRITE setArtSize NOTIFY spriteChanged)
    /*! Null, Ready or Error, so QML can fall back to another source. */
    Q_PROPERTY(Status status READ status NOTIFY spriteChanged)

public:
    enum Status {
        Null,
        Ready,
        Error,
    };
    Q_ENUM(Status)

    explicit PixelSkySprite(QQuickItem *parent = nullptr);

    [[nodiscard]] QUrl source() const { return m_source; }
    [[nodiscard]] int artSize() const { return m_artSize; }
    [[nodiscard]] Status status() const { return m_status; }
    void setSource(const QUrl &source);
    void setArtSize(int size);

Q_SIGNALS:
    void spriteChanged();

protected:
    [[nodiscard]] QImage render() const override { return m_decoded; }

private:
    void reload();

    QUrl m_source;
    int m_artSize = 16;
    Status m_status = Null;
    QImage m_decoded;
};
